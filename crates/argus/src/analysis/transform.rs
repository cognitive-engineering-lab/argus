use index_vec::IndexVec;
use rustc_data_structures::fx::{
  FxHashMap as HashMap, FxHashSet as HashSet, FxIndexMap,
};
use rustc_hir::{self as hir, intravisit::Map, BodyId, HirId};
use rustc_infer::{
  infer::{canonical::OriginalQueryValues, InferCtxt, InferOk},
  traits::{self, ObligationCauseCode, PredicateObligation},
};
use rustc_middle::ty::{
  self, ParamEnvAnd, ToPredicate, Ty, TyCtxt, TypeckResults,
};
use rustc_span::Span;
use rustc_utils::source_map::range::CharRange;

use super::{
  hir::{self as hier_hir, Bin, BinKind},
  tls::{SynIdx, UODIdx},
  EvaluationResult,
};
use crate::{
  ext::{EvaluationResultExt, InferCtxtExt, PredicateExt, TyCtxtExt},
  types::{intermediate::*, *},
};

pub fn compute_provenance<'tcx>(
  infcx: &InferCtxt<'tcx>,
  obligation: &PredicateObligation<'tcx>,
  result: EvaluationResult,
  dataid: Option<UODIdx>,
  synid: Option<SynIdx>,
) -> Provenance<Obligation> {
  let Some(ldef_id) = infcx.body_id() else {
    unreachable!("argus analysis should only happen on local bodies");
  };

  let hir = infcx.tcx.hir();
  let fdata = infcx.bless_fulfilled(obligation, result, synid.is_some());

  // If the span is coming from a macro, point to the callsite.
  let callsite_cause_span = fdata.obligation.cause.span.source_callsite();
  let body_id = hir.body_owned_by(ldef_id);
  let hir_id = hier_hir::find_most_enclosing_node(
    &infcx.tcx,
    body_id,
    callsite_cause_span,
  )
  .unwrap_or_else(|| hir.body_owner(body_id));

  Provenance {
    hir_id,
    full_data: dataid,
    synthetic_data: synid,
    it: infcx.erase_non_local_data(body_id, fdata),
  }
}

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
  let mut obligations_idx = IndexVec::<ObligationIdx, _>::default();

  let obligations_with_idx = obligations
    .into_iter()
    .map(|prov| {
      let hash_only = prov.map(|p| p.hash);
      let idx = obligations_idx.push(prov.forget());
      hash_only.map(|_| idx)
    })
    .collect::<Vec<_>>();

  let mut builder = ObligationsBuilder {
    tcx,
    body_id,
    typeck_results,

    raw_obligations: obligations_idx,
    obligations: obligations_with_idx,
    full_data: obligation_data,
    synthetic_data,
    reported_trait_errors,

    exprs_to_hir_id: Default::default(),
    ambiguity_errors: Default::default(),
    trait_errors: Default::default(),
    exprs: Default::default(),
    method_lookups: Default::default(),
  };

  builder.sort_bins(bins);
  builder.relate_trait_bound();

  let hir = tcx.hir();
  let source_map = tcx.sess.source_map();
  let body_range =
    CharRange::from_span(hir.body(body_id).value.span, source_map)
      .expect("Couldn't get body range");

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
  raw_obligations: IndexVec<ObligationIdx, Obligation>,
  obligations: Vec<Provenance<ObligationIdx>>,
  full_data: &'a ObligationQueriesInBody<'tcx>,
  synthetic_data: &'a mut SyntheticQueriesInBody<'tcx>,
  typeck_results: &'tcx TypeckResults<'tcx>,
  reported_trait_errors: &'a FxIndexMap<Span, Vec<ObligationHash>>,

  // Structures to be filled in
  exprs_to_hir_id: HashMap<ExprIdx, HirId>,
  ambiguity_errors: HashSet<ExprIdx>,
  trait_errors: HashSet<ExprIdx>,
  exprs: IndexVec<ExprIdx, Expr>,
  method_lookups: IndexVec<MethodLookupIdx, MethodLookup>,
}

impl<'a, 'tcx: 'a> ObligationsBuilder<'a, 'tcx> {
  pub(self) fn sort_bins(&mut self, bins: Vec<Bin>) {
    use ExprKind::*;

    let hir = self.tcx.hir();
    let source_map = self.tcx.sess.source_map();

    for Bin {
      hir_id,
      obligations,
      kind,
    } in bins
    {
      let span = hir.span_with_body(hir_id);
      if let Some((range, snippet)) =
        CharRange::from_span(span, source_map).ok().and_then(|r| {
          let snip = source_map
            .span_to_snippet(span)
            .unwrap_or_else(|_| String::from("{unknown snippet}"));
          Some((r, snip))
        })
      {
        let mut ambiguous_call = false;
        let kind = match kind {
          BinKind::Misc => Misc,
          BinKind::CallableExpr => CallableExpr,
          BinKind::CallArg => CallArg,
          BinKind::Call => Call,
          BinKind::MethodReceiver => MethodReceiver,
          BinKind::MethodCall => {
            let Some(hir::Node::Expr(
              call_expr @ hir::Expr {
                kind: hir::ExprKind::MethodCall(segment, recvr, args, call_span),
                ..
              },
            )) = hir.find(hir_id)
            else {
              unreachable!(
                "Bin kind `MethodCall` for non `ExprKind::MethodCall` {:?}",
                hir.node_to_string(hir_id)
              );
            };

            if let Some((idx, error_recvr, error_call)) = self
              .relate_method_call(
                call_expr,
                segment,
                recvr,
                args,
                *call_span,
                &obligations,
              )
            {
              ambiguous_call = error_recvr || error_call;
              MethodCall {
                data: idx,
                error_recvr,
              }
            } else {
              Misc
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
        if ambiguous_call {
          self.ambiguity_errors.insert(expr_idx);
        }
      } else {
        log::error!(
          "failed to get range for HIR: {}",
          hir.node_to_string(hir_id)
        );
      }
    }
  }

  // FIXME: this isn't efficient, but the number of obligations per
  // body isn't large, so shouldnt' be an issue.
  fn relate_trait_bound(&mut self) {
    // 1. take the expressions from the "reported_trait_errors" and find
    //    all the expressions that they correspond to. we should also
    //    maintain the order in which they are reported and use this
    //    sorting to present errors.
    for (span, predicates) in self.reported_trait_errors.iter() {
      let Some(this_id) =
        hier_hir::find_most_enclosing_node(&self.tcx, self.body_id, *span)
      else {
        log::error!("reported error doesn't have an associated span ...");
        continue;
      };

      let matching_expressions = self
        .exprs_to_hir_id
        .iter()
        .filter(|(_, that_id)| self.tcx.is_parent_of(**that_id, this_id))
        .collect::<Vec<_>>();

      let Some((expr_id, hir_id)) =
        matching_expressions.iter().copied().find(|(_, this_id)| {
          matching_expressions
            .iter()
            .all(|(_, that_id)| self.tcx.is_parent_of(**that_id, **this_id))
        })
      else {
        log::error!(
          "failed to find most enclosing hir id for {:?}",
          matching_expressions
        );
        continue;
      };

      // Mark the found Expr as containing an error.
      self.trait_errors.insert(*expr_id);

      // Sort the Expr obligations according to the reported order.
      let expr_obligations = &mut self.exprs[*expr_id].obligations;
      let num_errs = predicates.len();
      expr_obligations.sort_by_key(|&obl_idx| {
        let obl = &self.raw_obligations[obl_idx];
        let obl_hash = obl.hash;
        let obl_is_certain = obl.result.is_certain();
        predicates
          .iter()
          .position(|&h| h == obl_hash)
          .unwrap_or_else(|| {
            if obl_is_certain {
              // push successful obligations down
              num_errs + 1
            } else {
              num_errs
            }
          })
      })
    }

    // 2. we also need to search for expressions that are "ambiguous," these
    //    don't always have associated reported errors. my current thoughts to
    //    do this are to find obligations that are unsuccessful and have a
    //    concrete obligations code.
    //
    // TODO: this isn't quite doing what I want. We need a way to figure
    // out which obligations are "reruns" of a previous goal, and
    // then remove the prior 'ambiguous' answer from the list.
    //
    // let is_important_failed_query = |obl_idx: ObligationIdx| {
    //   use rustc_infer::traits::ObligationCauseCode::*;
    //   if let Some(prov) = self.obligations.iter().find(|p| ***p == obl_idx)
    //     && let Some(uodidx) = prov.full_data
    //   {
    //     let full_data = self.full_data.get(uodidx);
    //     !(full_data.result.is_certain()
    //       || matches!(full_data.obligation.cause.code(), MiscObligation))
    //   } else {
    //     false
    //   }
    // };
    // let lift_failed_obligations = |v: &mut Vec<ObligationIdx>| {
    //   v.sort_by_key(|&idx| {
    //     if self.raw_obligations[idx].result.is_certain() {
    //       1
    //     } else {
    //       0
    //     }
    //   })
    // };
    // let unmarked_exprs = self
    //   .exprs
    //   .iter_mut_enumerated()
    //   .filter(|(id, _)| !self.ambiguity_errors.contains(id));
    // for (expr_id, expr) in unmarked_exprs {
    //   let contains_failed = expr
    //     .obligations
    //     .iter()
    //     .copied()
    //     .any(is_important_failed_query);
    //   if contains_failed {
    //     self.trait_errors.insert(expr_id);
    //   }
    //   lift_failed_obligations(&mut expr.obligations)
    // }
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
        let Some(fdata) = self.obligations[idx]
          .full_data
          .map(|fdidx| self.full_data.get(fdidx))
        else {
          return None;
        };

        let is_necessary = fdata.obligation.predicate.is_trait_predicate() &&
        fdata
          .infcx
          .obligation_necessity(&fdata.obligation)
          .is_necessary(fdata.result)
          // TODO: Obligations for method calls are registered
          // usder 'misc,' this of course can change. Find a better
          // way to gather the attempted traits!
          && matches!(
            fdata.obligation.cause.code(),
            ObligationCauseCode::MiscObligation
          );

        is_necessary.then(|| {
          (idx, expect_trait_ref(&fdata.obligation.predicate).def_id())
        })
      })
      .unzip();

    let mut param_env = None;
    for &idx in &necessary_queries {
      let query = self.obligations[idx]
        .full_data
        .map(|fdidx| self.full_data.get(fdidx))
        .unwrap();

      if let Some(pe) = param_env
        && pe == query.obligation.param_env
      {
        log::warn!(
          "param environments are expected to match {:?} != {:?}",
          pe,
          query.obligation.param_env
        );
      } else {
        param_env = Some(query.obligation.param_env);
      }
    }

    let (full_query_idx, query) =
      necessary_queries.first().and_then(|&idx| {
        self.obligations[idx]
          .full_data
          .map(|fdidx| (fdidx, self.full_data.get(fdidx)))
      })?;

    let infcx = &query.infcx;
    let o = &query.obligation;
    let self_ty = expect_trait_ref(&o.predicate).self_ty().skip_binder();
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

    let ty_with_ref = move |ty: Ty<'tcx>| {
      Ty::new_ref(tcx, region, ty::TypeAndMut {
        ty,
        mutbl: hir::Mutability::Not,
      })
    };

    let ty_with_mut_ref = move |ty: Ty<'tcx>| {
      Ty::new_ref(tcx, region, ty::TypeAndMut {
        ty,
        mutbl: hir::Mutability::Mut,
      })
    };

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
    for ty_adjust in ty_mutators.into_iter() {
      let mut method_steps = Vec::default();
      for step in steps.steps.iter() {
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
        for trait_ref in trait_candidates.iter() {
          let trait_ref = trait_ref.with_self_ty(tcx, self_ty);

          let predicate: ty::Predicate<'tcx> =
            ty::Binder::dummy(trait_ref).to_predicate(self.tcx);
          let obligation =
            traits::Obligation::new(tcx, o.cause.clone(), param_env, predicate);

          let res = infcx.evaluate_obligation(&obligation);

          let syn_id = self.synthetic_data.add(SyntheticData {
            full_data: full_query_idx,
            obligation: obligation.clone(),
            result: res,
          });

          let with_provenance = compute_provenance(
            &infcx, // HACK very bad, get rid
            &obligation,
            res,
            None,
            Some(syn_id), // TODO:
          );

          trait_predicates
            .push(self.raw_obligations.push(with_provenance.forget()))
        }

        method_steps.push(MethodStep {
          recvr_ty: step,
          trait_predicates,
        })
      }

      table.extend(method_steps);
    }

    Some((
      self.method_lookups.push(MethodLookup {
        table,
        candidates: ExtensionCandidates::new(infcx, trait_candidates),
      }),
      error_recvr,
      error_call,
    ))
  }
}

fn expect_trait_ref<'tcx>(
  p: &ty::Predicate<'tcx>,
) -> ty::Binder<'tcx, ty::TraitPredicate<'tcx>> {
  p.kind().map_bound(|f| {
    let ty::PredicateKind::Clause(ty::ClauseKind::Trait(tr)) = f else {
      unreachable!();
    };
    tr
  })
}
