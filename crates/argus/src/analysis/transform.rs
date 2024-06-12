use index_vec::IndexVec;
use indexmap::IndexSet;
use itertools::Itertools;
use rustc_data_structures::fx::{FxHashMap as HashMap, FxIndexMap};
use rustc_hir::{self as hir, intravisit::Map, BodyId, HirId};
use rustc_infer::{
  infer::{canonical::OriginalQueryValues, InferCtxt, InferOk},
  traits::{self, PredicateObligation},
};
use rustc_middle::ty::{self, ParamEnvAnd, Ty, TyCtxt, TypeckResults, Upcast};
use rustc_span::Span;
use rustc_utils::source_map::{range::CharRange, span::SpanExt};

use super::{
  hir::{self as hier_hir, Bin, BinKind},
  tls::UODIdx,
  EvaluationResult,
};
use crate::{
  ext::{InferCtxtExt, PredicateExt, TyCtxtExt, TypeckResultsExt},
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
  let fdata = infcx.bless_fulfilled(obligation, result, false);
  // If the span is coming from a macro, point to the callsite.
  let callsite_cause_span =
    infcx.tcx.to_local(body_id, fdata.obligation.cause.span);
  let hir_id =
    hier_hir::find_most_enclosing_node(infcx.tcx, body_id, callsite_cause_span)
      .unwrap_or_else(|| hir.body_owner(body_id));

  Provenance {
    hir_id,
    full_data: dataid,
    synthetic_data: None,
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
pub fn transform<'a, 'tcx: 'a>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  typeck_results: &'tcx TypeckResults<'tcx>,
  obligations: Vec<Provenance<Obligation>>,
  obligation_data: &ObligationQueriesInBody<'tcx>,
  synthetic_data: &mut SyntheticQueriesInBody<'tcx>,
  reported_trait_errors: &FxIndexMap<Span, Vec<ObligationHash>>,
  bins: Vec<Bin>,
) -> ObligationsInBody {
  #[cfg(debug_assertions)]
  {
    debug_assert!(synthetic_data.is_empty(), "synthetic data not empty");
    debug_assert!(
      obligations.iter().all(|p| !p.is_synthetic),
      "synthetic obligations exist before method call table construction"
    );
  }

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
    synthetic_data,
    reported_trait_errors,

    exprs_to_hir_id: HashMap::default(),
    ambiguity_errors: IndexSet::default(),
    trait_errors: Vec::default(),
    exprs: IndexVec::default(),
    method_lookups: IndexVec::default(),
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
    builder.method_lookups,
  )
}

struct ObligationsBuilder<'a, 'tcx: 'a> {
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  body_span: Span,
  full_data: &'a ObligationQueriesInBody<'tcx>,
  typeck_results: &'tcx TypeckResults<'tcx>,
  reported_trait_errors: &'a FxIndexMap<Span, Vec<ObligationHash>>,

  // Mutable for adding synthetic data
  obligations: &'a Vec<Provenance<ObligationIdx>>,
  synthetic_data: &'a mut SyntheticQueriesInBody<'tcx>,

  // Structures to be filled in
  raw_obligations: IndexVec<ObligationIdx, Obligation>,
  exprs_to_hir_id: HashMap<ExprIdx, HirId>,
  ambiguity_errors: IndexSet<AmbiguityError>,
  trait_errors: Vec<TraitError>,
  exprs: IndexVec<ExprIdx, Expr>,
  method_lookups: IndexVec<MethodLookupIdx, MethodLookup>,
}

impl<'a, 'tcx: 'a> ObligationsBuilder<'a, 'tcx> {
  fn to_local(&self, span: Span) -> Span {
    span.as_local(self.body_span).unwrap_or(span)
  }

  fn local_snip(&self, span: Span) -> String {
    let source_map = self.tcx.sess.source_map();
    source_map
      .span_to_snippet(span)
      .unwrap_or_else(|_| String::from("{unknown snippet}"))
  }

  fn sort_bins(&mut self, bins: Vec<Bin>) {
    use ExprKind as EK;

    let hir = self.tcx.hir();
    let source_map = self.tcx.sess.source_map();
    for bin in bins {
      let Bin {
        hir_id,
        obligations,
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
      let mut ambiguous_call = None;
      let kind = match kind {
        BinKind::Misc => EK::Misc,
        BinKind::CallableExpr => EK::CallableExpr,
        BinKind::CallArg => EK::CallArg,
        BinKind::Call => EK::Call,
        BinKind::MethodReceiver => EK::MethodReceiver,
        BinKind::MethodCall => {
          let hir::Node::Expr(
            call_expr @ hir::Expr {
              kind: hir::ExprKind::MethodCall(segment, recvr, args, call_span),
              ..
            },
          ) = hir.hir_node(hir_id)
          else {
            unreachable!(
              "bin kind is method call, but node is not method call"
            );
          };

          if let Some((idx, error_recvr, error_call)) = self.relate_method_call(
            call_expr,
            segment,
            recvr,
            args,
            *call_span,
            &obligations,
          ) {
            if error_recvr || error_call {
              ambiguous_call = Some(call_span);
            }

            EK::MethodCall {
              data: idx,
              error_recvr,
            }
          } else {
            log::warn!(
              "failed to build method call table for {}",
              self.tcx.hir().node_to_string(call_expr.hir_id)
            );
            EK::Misc
          }
        }
      };

      let obligations = obligations
        .into_iter()
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
      if let Some(call_span) = ambiguous_call {
        let range = CharRange::from_span(*call_span, source_map)
          .expect("failed to get range for ambiguous call");
        self.ambiguity_errors.insert(AmbiguityError {
          idx: expr_idx,
          range,
        });
      }
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
      } else {
        log::error!(
          "failed to find root expression for {span:?} {predicates:?}"
        );
      }

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

  // 1. build the method call table (see ambiguous / )
  fn relate_method_call<'hir>(
    &mut self,
    // Expr of the entire call expression
    call_expr: &'hir hir::Expr<'hir>,
    // The method segment
    _segment: &'hir hir::PathSegment<'hir>,
    // Call receiver
    recvr: &'hir hir::Expr<'hir>,
    // Call arguments
    _args: &'hir [hir::Expr<'hir>],
    // Call expression span
    call_span: Span,
    // FIXME: type the `usize` here,
    obligations: &[usize],
  ) -> Option<(MethodLookupIdx, bool, bool)> {
    panic!("We shouldn't be computing synthetic goals rn");

    // Given that the receiver is of type error, we can tell users
    // to annotate the receiver type if they want to get "better"
    // error messages (potentially).
    let error_recvr = matches!(
      self.typeck_results.expr_ty(recvr).kind(),
      ty::TyKind::Error(..)
    );
    let error_call = matches!(
      self.typeck_results.expr_ty(call_expr).kind(),
      ty::TyKind::Error(..)
    );

    let (necessary_queries, trait_candidates): (Vec<_>, Vec<_>) = obligations
      .iter()
      .filter_map(|&idx| {
        let fdata = self.obligations[idx]
          .full_data
          .map(|fdidx| self.full_data.get(fdidx))?;

        if fdata.obligation.predicate.is_trait_predicate() {
          log::info!(
            "Predicate is a trait predicate {:?}",
            fdata.obligation.predicate
          );
        }

        // Bounds for extension method calls are always trait predicates.
        let tp = fdata.obligation.predicate.as_trait_predicate()?;
        Some((idx, tp.def_id()))
      })
      .unzip();

    let trait_candidates =
      trait_candidates.into_iter().unique().collect::<Vec<_>>();

    let mut param_env = None;
    for &idx in &necessary_queries {
      let query = self.obligations[idx]
        .full_data
        .map(|fdidx| self.full_data.get(fdidx))
        .unwrap();

      if let Some(pe) = param_env {
        if pe != query.obligation.param_env {
          log::error!(
            "param environments are expected to match {:?} != {:?}",
            pe,
            query.obligation.param_env
          );
        }
      } else {
        param_env = Some(query.obligation.param_env);
      }
    }

    let Some((full_query_idx, query)) =
      necessary_queries.first().and_then(|&idx| {
        self.obligations[idx]
          .full_data
          .map(|fdidx| (fdidx, self.full_data.get(fdidx)))
      })
    else {
      log::warn!("necessary queries empty! {:?}", necessary_queries);
      return None;
    };

    let infcx = &query.infcx;
    let o = &query.obligation;
    let self_ty = o
      .predicate
      .as_trait_predicate()
      .expect("trait predicate")
      .self_ty()
      .skip_binder();
    let param_env = o.param_env;

    let mut original_values = OriginalQueryValues::default();
    let param_env_and_self_ty = infcx.canonicalize_query(
      ParamEnvAnd {
        param_env,
        value: self_ty,
      },
      &mut original_values,
    );

    let tcx = self.tcx;
    let region = tcx.lifetimes.re_erased;
    let steps = tcx.method_autoderef_steps(param_env_and_self_ty);

    let ty_id = |ty: Ty<'tcx>| ty;

    let ty_with_ref =
      move |ty: Ty<'tcx>| Ty::new_ref(tcx, region, ty, hir::Mutability::Not);

    let ty_with_mut_ref =
      move |ty: Ty<'tcx>| Ty::new_ref(tcx, region, ty, hir::Mutability::Mut);

    // TODO: rustc also considers raw pointers, ignoring for now ...
    let ty_mutators: Vec<&dyn Fn(Ty<'tcx>) -> Ty<'tcx>> =
      vec![&ty_id, &ty_with_ref, &ty_with_mut_ref];

    let trait_candidates = trait_candidates
      .into_iter()
      .map(|trait_def_id| {
        let trait_args = infcx.fresh_args_for_item(call_span, trait_def_id);
        ty::TraitRef::new(tcx, trait_def_id, trait_args)
      })
      .collect::<Vec<_>>();

    let mut table = Vec::default();

    infcx.probe(|_| {
      for ty_adjust in ty_mutators {
        let mut method_steps = Vec::default();
        for step in steps.steps {
          let InferOk {
            value: self_ty,
            obligations: _,
          } = infcx
            .instantiate_query_response_and_region_obligations(
              &traits::ObligationCause::misc(call_span, o.cause.body_id),
              param_env,
              &original_values,
              &step.self_ty,
            )
            .unwrap_or_else(|_| unreachable!("whelp, that didn't work :("));

          let self_ty = ty_adjust(self_ty);
          let step = ReceiverAdjStep::new(infcx, self_ty);

          let mut trait_predicates = Vec::default();
          for trait_ref in &trait_candidates {
            let trait_ref = trait_ref.with_self_ty(tcx, self_ty);

            let predicate: ty::Predicate<'tcx> = trait_ref.upcast(self.tcx);
            let obligation = traits::Obligation::new(
              tcx,
              o.cause.clone(),
              param_env,
              predicate,
            );

            infcx.probe(|_| {
              let res = infcx.evaluate_obligation(&obligation);

              let mut with_provenance =
                compute_provenance(self.body_id, infcx, &obligation, res, None);

              let syn_id = self.synthetic_data.add(SyntheticData {
                full_data: full_query_idx,
                hash: with_provenance.hash,
                obligation: obligation.clone(),
                infcx: infcx.fork(),
              });

              with_provenance.mark_as_synthetic(syn_id);

              trait_predicates
                .push(self.raw_obligations.push(with_provenance.forget()));
            });

            property_is_ok!(
              self.is_valid(),
              "obligation invalidated the builder: {obligation:?}"
            );
          }

          method_steps.push(MethodStep {
            recvr_ty: step,
            trait_predicates,
          });
        }

        table.extend(method_steps);
      }
    });

    Some((
      self.method_lookups.push(MethodLookup {
        table,
        candidates: ExtensionCandidates::new(infcx, trait_candidates),
      }),
      error_recvr,
      error_call,
    ))
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
      if obl.is_synthetic {
        // assert that synthetic obligation exists
        let exists = self
          .synthetic_data
          .iter()
          .any(|sdata| obl.hash == sdata.hash);

        anyhow::ensure!(exists, "synthetic data not found for {:?}", obl);
      } else if matches!(obl.necessity, ObligationNecessity::Yes)
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
