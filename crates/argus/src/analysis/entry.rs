//! Code that relates two pieces of data, or computes the
//! rleationships between large structures.

use anyhow::{anyhow, Result};
use fluid_let::fluid_let;
use rustc_hir::BodyId;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::ty::{Predicate, TyCtxt, TypeckResults};
use rustc_trait_selection::traits::solve::Goal;
use rustc_utils::source_map::range::CharRange;
use serde::Serialize;

use crate::{
  analysis::{
    hir,
    tls::{self},
    transform, EvaluationResult, OBLIGATION_TARGET,
  },
  ext::InferCtxtExt,
  proof_tree::serialize::serialize_proof_tree,
  serialize::{serialize_to_value, ty::PredicateDef},
  types::{
    intermediate::{
      ErrorAssemblyCtx, ObligationQueriesInBody, SyntheticQueriesInBody,
    }, ObligationsInBody,
  },
};

fluid_let! {
  pub static INSPECTING: bool;
}

macro_rules! guard_inspection {
  () => {{
    if INSPECTING.copied().unwrap_or(false) {
      return;
    }
  }};
}

// --------------------------------
// Rustc inspection points

#[derive(Serialize)]
struct PredWrapper<'a, 'tcx: 'a>(
  #[serde(with = "PredicateDef")] &'a Predicate<'tcx>,
);

pub fn process_obligation<'tcx>(
  infcx: &InferCtxt<'tcx>,
  obl: &PredicateObligation<'tcx>,
  result: EvaluationResult,
) {
  guard_inspection! {}

  let Some(_ldef_id) = infcx.body_id() else {
    log::warn!("Skipping obligation unassociated with local body {obl:?}");
    return;
  };

  // TODO: we need to figure out when to save the full data.
  // Saving it for every obligation will consume a ton of memory
  // and make the tool relatively slow, but I don't have a tight
  // enough metric as to when it should be ignored. NOTE: that
  // if the full data doesn't get stored for every obligation, make
  // sure that usages of `provenance.full_data` are guarded, as
  // some currently use `.unwrap()`.
  let dataid = Some(tls::unsafe_store_data(infcx, obl, result));

  let obligation =
    transform::compute_provenance(infcx, obl, result, dataid, None);

  tls::store_obligation(obligation);
}

// TODO: we now need to handle synthetic predicates, those
// are the predicates we simulate for method lookup. If the
// target is synthetic we'll just have to handle all obligations
// as from the previous case, and then search through the associated
// method calls for the obligation.
pub fn process_obligation_for_tree<'tcx>(
  infcx: &InferCtxt<'tcx>,
  obl: &PredicateObligation<'tcx>,
  result: EvaluationResult,
) {
  let _target = OBLIGATION_TARGET.get(|target| {
    let target = target.unwrap();

    // A synthetic target requires that we do the method call queries.
    if target.is_synthetic {
      log::debug!("Skipping synthetic obligation");
      process_obligation(infcx, obl, result);
      return;
    }

    // Must go after the synthetic check.
    guard_inspection! {}

    let hash = infcx.predicate_hash(&obl.predicate);

    if hash.as_u64() != *target.hash {
      return;
    }

    if let Some(stree) = generate_tree(infcx, obl) {
      tls::store_tree(stree);
    }
  });
}

// --------------------------------
// Output builders

/// Retrieve *all* obligations processed from rustc.
pub fn build_obligations_output<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  typeck_results: &'tcx TypeckResults<'tcx>,
) -> Result<serde_json::Value> {
  let oib = build_obligations_in_body(tcx, body_id, typeck_results).1;
  serde_json::to_value(&oib).map_err(|e| anyhow!(e))
}

// TODO: if we are looking for the tree of a synthetic obligation
// then we will actually need to do some computation here.
pub fn build_tree_output<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  typeck_results: &'tcx TypeckResults<'tcx>,
) -> Option<serde_json::Value> {
  OBLIGATION_TARGET.get(|target| {
    let target = target?;
    match (tls::take_tree(), target.is_synthetic) {
      (o @ Some(_), false) => o,
      (o @ None, false) => {
        log::error!("failed to find tree for obligation target {:?}", target);
        o
      }
      (_, true) => {
        let (data, _) = build_obligations_in_body(tcx, body_id, typeck_results);

        data.synthetic.into_iter().find_map(|sdata| {
          let full_data = &data.obligations.get(sdata.full_data);
          let infcx = &full_data.infcx;
          let hash = infcx.predicate_hash(&sdata.obligation.predicate);
          if hash.as_u64() == *target.hash {
            generate_tree(infcx, &sdata.obligation)
          } else {
            None
          }
        })
      }
    }
  })
}

fn generate_tree<'tcx>(
  infcx: &InferCtxt<'tcx>,
  obligation: &PredicateObligation<'tcx>,
) -> Option<serde_json::Value> {
  let goal = Goal {
    predicate: obligation.predicate,
    param_env: obligation.param_env,
  };

  let item_def_id = infcx.body_id()?.to_def_id();
  serialize_proof_tree(goal, infcx, item_def_id)
    .and_then(|serial_tree| serialize_to_value(infcx, &serial_tree).ok())
}

struct FullData<'tcx> {
  obligations: ObligationQueriesInBody<'tcx>,
  synthetic: SyntheticQueriesInBody<'tcx>,
}

fn build_obligations_in_body<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  typeck_results: &'tcx TypeckResults<'tcx>,
) -> (FullData<'tcx>, ObligationsInBody) {
  let hir = tcx.hir();
  let source_map = tcx.sess.source_map();
  let _name = hir.opt_name(hir.body_owner(body_id));
  let body = hir.body(body_id);
  let _body_range = CharRange::from_span(body.value.span, source_map)
    .expect("Couldn't get body range");

  let obligations = tls::take_obligations();
  let obligation_data = tls::unsafe_take_data();

  let obligation_data = ObligationQueriesInBody::new(obligation_data);
  let mut synthetic_data = SyntheticQueriesInBody::new();

  // let bound_info = tls::take_trait_error_info();
  let ctx = ErrorAssemblyCtx {
    tcx,
    body_id,
    typeck_results,
    obligations: &obligations,
    obligation_data: &obligation_data,
  };

  let bins = hir::associate_obligations_nodes(&ctx);
  let oib = transform::transform(
    tcx,
    body_id,
    typeck_results,
    obligations,
    &obligation_data,
    &mut synthetic_data,
    bins,
  );

  (
    FullData {
      obligations: obligation_data,
      synthetic: synthetic_data,
    },
    oib,
  )
}
