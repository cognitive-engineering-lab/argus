use argus_ext::{
  infer::InferCtxtExt,
  ty::{
    retain_error_sources, retain_method_calls, TyCtxtExt, TypeckResultsExt,
  },
  utils::SpanExt as ArgusSpanExt,
};
use index_vec::IndexVec;
use indexmap::IndexSet;
use rustc_data_structures::fx::{FxHashMap as HashMap, FxIndexMap};
use rustc_hir::{BodyId, HirId};
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::ty::{TyCtxt, TypeckResults};
use rustc_span::Span;
use rustc_utils::source_map::{range::CharRange, span::SpanExt};

use super::{
  hir::{self as hier_hir, Bin, BinKind},
  tls::UODIdx,
  EvaluationResult,
};
use crate::{
  ext::InferCtxtExt as LocalInferCtxtExt,
  types::{intermediate::*, *},
};

macro_rules! property_is_ok {
  ($prop:expr, $t:tt) => {{
    #[cfg(any(feature = "testing", debug_assertions))]
    {
      let it = $prop;
      if !it.is_ok() {
        log::error!("property {} failed: {:?}", stringify!($prop), it);
        assert!(false, $t);
      }
    }
  }};
}

pub fn compute_provenance<'tcx>(
  body_id: BodyId,
  infcx: &InferCtxt<'tcx>,
  obligation: &PredicateObligation<'tcx>,
  result: EvaluationResult,
  dataid: Option<UODIdx>,
) -> Provenance<Obligation> {
  let hir = infcx.tcx.hir();
  let fdata = infcx.bless_fulfilled(obligation, result);
  // If the span is coming from a macro, point to the callsite.
  let callsite_cause_span =
    infcx.tcx.to_local(body_id, fdata.obligation.cause.span);
  let hir_id =
    hier_hir::find_most_enclosing_node(infcx.tcx, body_id, callsite_cause_span)
      .unwrap_or_else(|| hir.body_owner(body_id));

  Provenance {
    hir_id,
    full_data: dataid,
    it: infcx.erase_non_local_data(body_id, fdata),
  }
}

/// Transform a set of obligations into categorized trait errors.
///
/// `bins` coarsely associates obligations with HIR *expressions*. In
/// the future we should be even more precise and associate them with
/// arbitrary nodes. This is important because predicates like `WellFormed`
/// associate with signature types.
///
/// 1. Take the map of 'HIR ids -> obligations' and sort these into
///    expressions. This leaves us with a map of 'expressions -> hirids'.
/// 2. Look at the rustc reported trait errors and find the obligations in
///    out stack. This can be tricky because we have root goals but rustc
///    reports the lowest non-branching obligations. See `tree_search` for a
///    dirty way of estimating this.
/// 3. Relate "ambiguous" method calls. These don't get reported in the set of
///    rustc trait errors (not sure why) but we need to represent them as
///    trait errors.
#[allow(clippy::too_many_arguments)]
pub fn transform<'a, 'tcx: 'a>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  typeck_results: &'tcx TypeckResults<'tcx>,
  obligations: Vec<Provenance<Obligation>>,
  obligation_data: &FullData<'tcx>,
  reported_trait_errors: &FxIndexMap<Span, Vec<ObligationHash>>,
  bins: Vec<Bin>,
) -> ObligationsInBody {
  let mut obligations_idx = IndexVec::<ObligationIdx, _>::default();

  let obligations_with_idx = obligations
    .into_iter()
    .map(|prov| {
      let hash_only = prov.map(|p| p.hash);
      let idx = obligations_idx.push(prov.forget());
      hash_only.map(|_| idx)
    })
    .collect::<Vec<_>>();

  let hir = tcx.hir();
  let source_map = tcx.sess.source_map();
  let body_span = hir.body(body_id).value.span;
  let body_range = CharRange::from_span(body_span, source_map)
    .expect("Couldn't get body range");

  let mut builder = ObligationsBuilder {
    tcx,
    body_id,
    typeck_results,
    body_span,

    raw_obligations: obligations_idx,
    obligations: &obligations_with_idx,
    full_data: obligation_data,
    reported_trait_errors,

    exprs_to_hir_id: HashMap::default(),
    ambiguity_errors: IndexSet::default(),
    trait_errors: Vec::default(),
    exprs: IndexVec::default(),
  };

  builder.sort_bins(bins);
  property_is_ok!(builder.is_valid(), "builder is invalid");

  builder.relate_trait_bounds();
  property_is_ok!(builder.is_valid(), "builder is invalid");

  // Relating arbitrary errors in the HIR to failed obligations can overwhelm
  // guaranteed reported errors. We only want to build these when no other errors
  // where found but type-checking failed.
  if builder.trait_errors.is_empty()
    && builder.ambiguity_errors.is_empty()
    && builder.typeck_results.tainted_by_errors.is_some()
  {
    builder.relate_unreported_errors();
    property_is_ok!(builder.is_valid(), "builder is invalid");
  }

  let name = obligation_data.iter().next().map(|fdata| {
    (
      &fdata.infcx,
      tcx.hir().body_owner_def_id(body_id).to_def_id(),
    )
  });

  ObligationsInBody::new(
    name,
    body_range,
    builder.ambiguity_errors,
    builder.trait_errors,
    builder.raw_obligations,
    builder.exprs,
  )
}

struct ObligationsBuilder<'a, 'tcx: 'a> {
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  body_span: Span,
  full_data: &'a FullData<'tcx>,
  typeck_results: &'tcx TypeckResults<'tcx>,
  reported_trait_errors: &'a FxIndexMap<Span, Vec<ObligationHash>>,

  obligations: &'a Vec<Provenance<ObligationIdx>>,

  // Structures to be filled in
  raw_obligations: IndexVec<ObligationIdx, Obligation>,
  exprs_to_hir_id: HashMap<ExprIdx, HirId>,
  ambiguity_errors: IndexSet<AmbiguityError>,
  trait_errors: Vec<TraitError>,
  exprs: IndexVec<ExprIdx, Expr>,
}

impl<'a, 'tcx: 'a> ObligationsBuilder<'a, 'tcx> {
  fn to_local(&self, span: Span) -> Span {
    span.as_local(self.body_span).unwrap_or(span)
  }

  fn local_snip(&self, span: Span) -> String {
    let source_map = self.tcx.sess.source_map();
    span.sanitized_snippet(source_map)
  }

  fn sort_bins(&mut self, bins: Vec<Bin>) {
    use ExprKind as EK;

    let hir = self.tcx.hir();
    let source_map = self.tcx.sess.source_map();
    for bin in bins {
      let Bin {
        hir_id,
        mut obligations,
        kind,
      } = bin;
      let span = self.to_local(hir.span_with_body(hir_id));
      let snippet = self.local_snip(span);
      let Ok(range) = CharRange::from_span(span, source_map) else {
        log::error!(
          "failed to get range for HIR: {}",
          hir.node_to_string(hir_id)
        );
        continue;
      };

      log::debug!(
          "Sorting at\nrange:{range:?}\nhir_span: {:?}\nfrom_expansion: {}\nspan: {span:?}",
          hir.span_with_body(hir_id),
          hir.span_with_body(hir_id).from_expansion()
        );
      let kind = match kind {
        BinKind::Misc => EK::Misc,
        BinKind::CallableExpr => EK::CallableExpr,
        BinKind::CallArg => EK::CallArg,
        BinKind::Call => EK::Call,
      };

      // We can only filter obligations that have known provenance data, so just split the
      // others off and add them back in later.
      let idx = itertools::partition(&mut obligations, |i| {
        self.obligations[*i].full_data.is_some()
      });
      let obligations_no_data = obligations.split_off(idx);

      // Given a usize index get the `FullObligationData` expected for it.
      let gfdata = |i: usize| {
        self
          .full_data
          .get(self.obligations[i].full_data.expect("yikes"))
      };

      // Filter down the set of obligations as much as possible.
      //
      // 1. Remove obligations that shouldn't have been checked. (I.e., a failed
      // precondition dissalows it from succeeding.) Hopefully, in the future these
      // aren't even solved for.
      retain_error_sources(
        &mut obligations,
        |&i| gfdata(i).result,
        |&i| gfdata(i).obligation.predicate,
        |_| self.tcx,
        |&a, &b| a == b,
      );

      retain_method_calls(
        &mut obligations,
        |&i| gfdata(i).result,
        |&i| gfdata(i).obligation.predicate,
        |_| self.tcx,
        |&a, &b| a == b,
      );

      let obligations = obligations
        .into_iter()
        // marge back in indices without data
        .chain(obligations_no_data.into_iter())
        .map(|idx| *self.obligations[idx])
        .collect::<Vec<_>>();

      let is_body = hir_id == self.tcx.hir().body_owner(self.body_id);
      let expr_idx = self.exprs.push(Expr {
        range,
        snippet,
        obligations,
        kind,
        is_body,
      });

      self.exprs_to_hir_id.insert(expr_idx, hir_id);
    }
  }

  fn exact_predicate_search(
    &self,
    needle: ObligationHash,
  ) -> Option<ObligationIdx> {
    self
      .raw_obligations
      .iter_enumerated()
      .find_map(|(obl_id, obl)| (obl.hash == needle).then_some(obl_id))
  }

  fn shallow_tree_predicate_search(
    &self,
    needle: ObligationHash,
  ) -> Option<ObligationIdx> {
    self.obligations.iter().find_map(|prov| {
      let uoidx = prov.full_data?;
      let full_data = self.full_data.get(uoidx);
      tree_search::tree_contains_in_branchless(
        // something
        &full_data.infcx,
        // something
        &full_data.obligation,
        needle,
      )
      .then_some(prov.it)
    })
  }

  fn relate_trait_bounds(&mut self) {
    for (&span, predicates) in self.reported_trait_errors {
      let span = self.to_local(span);
      let range = CharRange::from_span(span, self.tcx.sess.source_map())
        .expect("failed to get range for reported trait error");

      log::debug!(
        "Relating trait bounds:\nrange {range:?}\nspan: {span:?}\n{predicates:#?}"
      );

      // Search for the obligation hash in our set of computed obligations.
      let predicates = predicates
        .iter()
        .filter_map(|&p| {
          self
            .exact_predicate_search(p)
            .or_else(|| self.shallow_tree_predicate_search(p))
            .map(|h| (h, p))
        })
        .collect::<Vec<_>>();

      // Associate these with an expression, first comes first served.
      let mut root_expr = None;
      'outer: for (expr_id, expr) in self.exprs.iter_enumerated() {
        for (p, _) in &predicates {
          if expr.obligations.contains(p) {
            root_expr = Some(expr_id);
            break 'outer;
          }
        }
      }

      if let Some(expr_id) = root_expr {
        let (_, hashes): (Vec<ObligationIdx>, _) =
          predicates.into_iter().unzip();

        self.trait_errors.push(TraitError {
          idx: expr_id,
          range,
          hashes,
        });
        continue;
      }

      log::error!("failed to find root expression for {span:?} {predicates:?}");

      // A predicate did not match exactly, now we're scrambling
      // to find an expression by span, and pick an obligation.
      let Some(err_hir_id) =
        hier_hir::find_most_enclosing_node(self.tcx, self.body_id, span)
      else {
        log::error!("reported error doesn't have an associated span ...");
        continue;
      };

      let parent_ids_of_error = self
        .exprs_to_hir_id
        .iter()
        .filter(|(_, expr_hir_id)| {
          self.tcx.is_parent_of(**expr_hir_id, err_hir_id)
        })
        .collect::<Vec<_>>();

      let Some((expr_id, _hir_id)) =
        parent_ids_of_error.iter().copied().find(|(_, this_id)| {
          // Find child-most expression that contains the error.
          parent_ids_of_error
            .iter()
            .all(|(_, that_id)| self.tcx.is_parent_of(**that_id, **this_id))
        })
      else {
        log::error!(
          "failed to find most enclosing hir id for {:?}",
          parent_ids_of_error
        );
        continue;
      };

      // Mark the found Expr as containing an error.
      self.trait_errors.push(TraitError {
        idx: *expr_id,
        range,
        hashes: vec![],
      });
    }
  }

  /// Find error nodes in the HIR and search for failed obligation failures in the node.
  fn relate_unreported_errors(&mut self) {
    // for all error nodes in the HIR, find a binned failure in that same node.
    for hir_id in self.typeck_results.error_nodes() {
      let Some((eid, _)) =
        self.exprs_to_hir_id.iter().find(|(_, hid)| **hid == hir_id)
      else {
        continue;
      };

      let expr = &self.exprs[*eid];
      let span = self.tcx.hir().span(hir_id);
      let range = CharRange::from_span(span, self.tcx.sess.source_map())
        .expect("failed to get range for reported trait error");

      let hashes = expr
        .obligations
        .iter()
        .filter_map(|&idx| {
          let obligation = &self.raw_obligations[idx];
          match obligation.result {
            Ok(..) => None,
            Err(..) => Some(obligation.hash),
          }
        })
        .collect::<Vec<_>>();

      self.trait_errors.push(TraitError {
        idx: *eid,
        range,
        hashes,
      });
    }
  }

  #[cfg(any(feature = "testing", debug_assertions))]
  fn is_valid(&self) -> anyhow::Result<()> {
    for obl in &self.raw_obligations {
      if matches!(obl.necessity, ObligationNecessity::Yes)
        || (matches!(obl.necessity, ObligationNecessity::OnError)
          && obl.result.is_err())
      {
        let exists = self.full_data.iter().any(|fdata| fdata.hash == obl.hash);

        anyhow::ensure!(exists, "full data not found for {:?}", obl);
      }
    }

    Ok(())
  }
}

mod tree_search {
  use std::ops::ControlFlow;

  use rustc_trait_selection::{
    solve::inspect::{InspectGoal, ProofTreeInferCtxtExt, ProofTreeVisitor},
    traits::solve::Goal,
  };

  use super::*;

  /// Search for the target obligation along the non-branching tree path.
  ///
  /// This is usefull if a predicate, reported as a trait error, does not
  /// match one of the stored roots. This can happen when the start of
  /// the "trait tree" is a stick, e.g.,
  ///
  /// ```text
  ///  Ty: TRAIT_0
  ///       |
  ///  Ty: Trait_1
  ///    /   \
  ///  ...   ...
  /// ```
  ///
  /// rustc will report that `Ty` doesn't implement `Trait_1`, even thought the root
  /// obligation was for `TRAIT_0`.
  pub(super) fn tree_contains_in_branchless<'tcx>(
    infcx: &InferCtxt<'tcx>,
    obligation: &PredicateObligation<'tcx>,
    needle: ObligationHash,
  ) -> bool {
    infcx.probe(|_| {
      let goal = Goal {
        predicate: obligation.predicate,
        param_env: obligation.param_env,
      };
      let mut finder = BranchlessSearch::new(needle);
      infcx.visit_proof_tree(goal, &mut finder);
      finder.was_found()
    })
  }

  struct BranchlessSearch {
    needle: ObligationHash,
    found: bool,
  }

  impl BranchlessSearch {
    fn new(needle: ObligationHash) -> Self {
      Self {
        needle,
        found: false,
      }
    }

    fn was_found(self) -> bool {
      self.found
    }
  }

  impl<'tcx> ProofTreeVisitor<'tcx> for BranchlessSearch {
    type Result = ControlFlow<()>;

    fn span(&self) -> Span {
      rustc_span::DUMMY_SP
    }

    fn visit_goal(&mut self, goal: &InspectGoal<'_, 'tcx>) -> Self::Result {
      let infcx = goal.infcx();
      let predicate = &goal.goal().predicate;
      let hash = infcx.predicate_hash(predicate).into();
      if self.needle == hash {
        self.found = true;
        return ControlFlow::Break(());
      }

      let candidates = goal.candidates();
      if 1 == candidates.len() {
        candidates[0].visit_nested_in_probe(self)
      } else {
        ControlFlow::Break(())
      }
    }
  }
}
