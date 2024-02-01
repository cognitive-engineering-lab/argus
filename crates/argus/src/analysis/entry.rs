//! Code that relates two pieces of data, or computes the
//! rleationships between large structures.

use anyhow::{anyhow, Result};
use fluid_let::fluid_let;
use rustc_data_structures::fx::FxHashMap as HashMap;
use rustc_hir::BodyId;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::ty::{Predicate, TyCtxt, TypeckResults};
use rustc_trait_selection::traits::solve::Goal;
use rustc_utils::source_map::range::CharRange;
use serde::Serialize;

use crate::{
  analysis::{
    hir,
    tls::{self, FullObligationData, UODIdx},
    transform, EvaluationResult, Provenance,
    OBLIGATION_TARGET,
  },
  ext::InferCtxtExt,
  proof_tree::serialize::serialize_proof_tree,
  serialize::{serialize_to_value, ty::PredicateDef},
  types::{Obligation},
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

  let obligation = transform::compute_provenance(infcx, obl, result, dataid);

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
  _result: EvaluationResult,
) {
  guard_inspection! {}

  OBLIGATION_TARGET.get(|target| {
    INSPECTING.set(true, || {
      let inner = move || {
        let target = target?;
        let hash = infcx.predicate_hash(&obl.predicate);

        if hash.as_u64() != *target.hash {
          return None;
        }

        let goal = Goal {
          predicate: obl.predicate,
          param_env: obl.param_env,
        };

        let item_def_id = infcx.body_id()?.to_def_id();
        let serial_tree = serialize_proof_tree(goal, infcx, item_def_id)?;

        serialize_to_value(infcx, &serial_tree).ok()
      };

      if let Some(stree) = inner() {
        tls::store_tree(stree);
      }
    })
  })
}

// --------------------------------
// Output builders

pub struct ErrorAssemblyCtx<'a, 'tcx: 'a> {
  pub tcx: TyCtxt<'tcx>,
  pub body_id: BodyId,
  pub typeck_results: &'tcx TypeckResults<'tcx>,
  pub obligations: &'a Vec<Provenance<Obligation>>,
  pub obligation_data: &'a ObligationQueriesInBody<'tcx>,
}

pub(crate) struct ObligationQueriesInBody<'tcx>(
  HashMap<UODIdx, FullObligationData<'tcx>>,
);

impl<'tcx> ObligationQueriesInBody<'tcx> {
  pub fn get(&self, idx: UODIdx) -> &FullObligationData<'tcx> {
    &self.0.get(&idx).unwrap()
  }
}

/// Retrieve *all* obligations processed from rustc.
pub fn build_obligations_output<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  typeck_results: &'tcx TypeckResults<'tcx>,
) -> Result<serde_json::Value> {
  

  let hir = tcx.hir();
  let source_map = tcx.sess.source_map();
  let _name = hir.opt_name(hir.body_owner(body_id));
  let body = hir.body(body_id);
  let _body_range = CharRange::from_span(body.value.span, source_map)
    .expect("Couldn't get body range");

  let mut obligations = tls::take_obligations();
  let obligation_data = obligations
    .iter_mut()
    .filter_map(|o| {
      let idx = o.full_data?;
      let v = tls::unsafe_take_data(idx)?;
      Some((idx, v))
    })
    .collect::<HashMap<_, _>>();

  let obligation_data = ObligationQueriesInBody(obligation_data);

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
    bins,
  );

  serde_json::to_value(&oib).map_err(|e| anyhow!(e))
}

// TODO: if we are looking for the tree of a synthetic obligation
// then we will actually need to do some computation here.
pub fn build_tree_output<'tcx>(
  _tcx: TyCtxt<'tcx>,
  _body_id: BodyId,
) -> Option<serde_json::Value> {
  tls::take_tree()
}
