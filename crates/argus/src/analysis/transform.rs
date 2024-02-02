use index_vec::IndexVec;

use rustc_data_structures::fx::FxHashSet as HashSet;
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
  EvaluationResult, Provenance,
};
use crate::{
  ext::{InferCtxtExt, PredicateExt},
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
  let fdata =
    infcx.bless_fulfilled(ldef_id, obligation, result, synid.is_some());

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
    it: infcx.erase_non_local_data(fdata),
  }
}

pub fn transform<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  typeck_results: &'tcx TypeckResults<'tcx>,
  obligations: Vec<Provenance<Obligation>>,
  obligation_data: &ObligationQueriesInBody<'tcx>,
  synthetic_data: &mut SyntheticQueriesInBody<'tcx>,
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

    ambiguity_errors: Default::default(),
    trait_errors: Default::default(),
    exprs: Default::default(),
    method_lookups: Default::default(),
  };

  builder.sort_bins(bins);

  let hir = tcx.hir();
  let source_map = tcx.sess.source_map();
  let name = hir.opt_name(hir.body_owner(body_id));
  let body = hir.body(body_id);
  let body_range = CharRange::from_span(body.value.span, source_map)
    .expect("Couldn't get body range");

  return ObligationsInBody {
    name,
    range: body_range,
    obligations: builder.raw_obligations,
    ambiguity_errors: builder.ambiguity_errors,
    trait_errors: builder.trait_errors,
    exprs: builder.exprs,
    method_lookups: builder.method_lookups,
  };
}

struct ObligationsBuilder<'a, 'tcx: 'a> {
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  raw_obligations: IndexVec<ObligationIdx, Obligation>,
  obligations: Vec<Provenance<ObligationIdx>>,
  full_data: &'a ObligationQueriesInBody<'tcx>,
  synthetic_data: &'a mut SyntheticQueriesInBody<'tcx>,
  typeck_results: &'tcx TypeckResults<'tcx>,

  // Structures to be filled in
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
      if let Some(range) = hir
        .opt_span(hir_id)
        .and_then(|span| CharRange::from_span(span, source_map).ok())
      {
        let kind = match kind {
          BinKind::Misc => Misc,
          BinKind::CallableExpr => CallableExpr,
          BinKind::CallArg => CallArg,
          BinKind::Call => Call,
          BinKind::MethodReceiver => MethodReceiver,
          BinKind::MethodCall => {
            let Some(hir::Node::Expr(hir::Expr {
              kind: hir::ExprKind::MethodCall(segment, recvr, args, call_span),
              ..
            })) = hir.find(hir_id)
            else {
              unreachable!(
                "Bin kind `MethodCall` for non `ExprKind::MethodCall` {:?}",
                hir.node_to_string(hir_id)
              );
            };

            if let Some((idx, error_recvr)) = self.relate_method_call(
              hir_id,
              segment,
              recvr,
              args,
              *call_span,
              &obligations,
            ) {
              MethodCall {
                data: idx,
                error_recvr,
              }
            } else {
              Misc // FIXME: remove this after debugging!
            }
          }
        };

        let obligations = obligations
          .into_iter()
          .map(|idx| self.obligations[idx].it)
          .collect::<HashSet<_>>();

        self.exprs.push(Expr {
          range,
          obligations,
          kind,
        });
      } else {
        log::error!(
          "failed to get range for HIR: {}",
          hir.node_to_string(hir_id)
        );
      }
    }
  }

  fn relate_trait_bound(&mut self) {
    // TODO: !
  }

  // TODO: for the method call we need to:
  //
  // 1. build the method call table (see ambiguous / )
  fn relate_method_call<'hir>(
    &mut self,
    // Id of the call e xpression (for debugging only)
    _hir_id: HirId,
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
  ) -> Option<(MethodLookupIdx, bool)> {
    let _hir = self.tcx.hir();

    // Given that the receiver is of type error, we can tell users
    // to annotate the receiver type if they want to get "better"
    // error messages (potentially).
    let error_recvr = matches!(
      self.typeck_results.expr_ty(recvr).kind(),
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
        for trait_def_id in trait_candidates.iter() {
          let trait_args = infcx.fresh_args_for_item(call_span, *trait_def_id);
          let trait_ref = ty::TraitRef::new(tcx, *trait_def_id, trait_args);

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
      self.method_lookups.push(MethodLookup { table }),
      error_recvr,
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
