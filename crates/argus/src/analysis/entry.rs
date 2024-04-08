//! Code that relates two pieces of data, or computes the
//! rleationships between large structures.

use anyhow::{anyhow, bail, Result};
use fluid_let::fluid_let;
use rustc_hir::BodyId;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::ty::{Predicate, TyCtxt, TypeckResults};
use rustc_trait_selection::traits::solve::Goal;
use serde::Serialize;

use crate::{
  analysis::{
    hir,
    tls::{self},
    transform, EvaluationResult, INCLUDE_SUCCESSES, OBLIGATION_TARGET,
  },
  ext::{EvaluationResultExt, InferCtxtExt},
  proof_tree::{serialize::serialize_proof_tree, SerializedTree},
  serialize::ty::PredicateDef,
  types::{
    intermediate::{
      ErrorAssemblyCtx, Forgettable, FullData, ObligationQueriesInBody,
      SyntheticQueriesInBody,
    },
    ObligationHash, ObligationNecessity, ObligationsInBody,
  },
};

fluid_let! {
  pub static INSPECTING: bool;
  pub static BODY_ID: BodyId;
}

macro_rules! guard_inspection {
  () => {{
    if INSPECTING.copied().unwrap_or(false) {
      return;
    }
    fluid_let::fluid_set!(INSPECTING, true);
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

  let Some(body_id) = BODY_ID.copied() else {
    return;
  };

  // Use this to get rid of any resolved inference variables,
  // these could have been resolved while trying to solve the obligation
  // and we want to present it as such to the user.
  let obl = &infcx.resolve_vars_if_possible(obl.clone());

  // HACK: Remove ambiguous obligations if a "stronger" result was found and
  // the predicate implies the  previous. This is necessary because we
  // can't (currently) distinguish between a subsequent solving attempt
  // of a previous obligation.
  if result.is_yes() || result.is_no() {
    tls::drain_implied_ambiguities(infcx, &obl);
  }

  if !INCLUDE_SUCCESSES.copied().unwrap_or(false) && result.is_yes() {
    log::debug!("Skipping successful obligation {obl:?}");
    return;
  }

  log::debug!("Processing obligation {obl:?}");

  let necessity = infcx.obligation_necessity(obl);
  let dataid = if matches!(necessity, ObligationNecessity::Yes)
    || (matches!(necessity, ObligationNecessity::OnError) && result.is_no())
  {
    Some(tls::unsafe_store_data(infcx, obl, result))
  } else {
    None
  };

  let obligation =
    transform::compute_provenance(body_id, infcx, obl, result, dataid);

  tls::store_obligation(obligation);

  tls::replace_reported_errors(infcx);
}

pub fn process_obligation_for_tree<'tcx>(
  infcx: &InferCtxt<'tcx>,
  obl: &PredicateObligation<'tcx>,
  result: EvaluationResult,
) {
  OBLIGATION_TARGET.get(|target| {
    let target = target.unwrap();

    // A synthetic target requires that we do the method call queries.
    if target.is_synthetic {
      log::debug!("Deferring synthetic obligation tree search");
      process_obligation(infcx, obl, result);
      return;
    }

    // Must go after the synthetic check.
    guard_inspection! {}

    // Use this to get rid of any resolved inference variables,
    // these could have been resolved while trying to solve the obligation
    // and we want to present it as such to the user.
    let obl = &infcx.resolve_vars_if_possible(obl.clone());

    let fdata = infcx.bless_fulfilled(obl, result, false);

    if fdata.hash != target.hash {
      return;
    }

    match generate_tree(infcx, obl) {
      Ok(stree) => tls::store_tree(stree),
      Err(e) => {
        log::error!("matching target tree not generated {e:?}");
      }
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
) -> Result<ObligationsInBody> {
  let (_, oib) = build_obligations_in_body(tcx, body_id, typeck_results);
  Ok(oib)
}

pub fn build_tree_output<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  typeck_results: &'tcx TypeckResults<'tcx>,
) -> Result<SerializedTree> {
  OBLIGATION_TARGET.get(|target| {
    let target = target.ok_or(anyhow!("missing target"))?;
    let (data, oib) = build_obligations_in_body(tcx, body_id, typeck_results);
    pick_tree(target.hash, target.is_synthetic, || (&*data, &oib))
  })
}

pub(crate) fn pick_tree<'a, 'tcx: 'a>(
  hash: ObligationHash,
  needs_search: bool,
  thunk: impl FnOnce() -> (&'a FullData<'tcx>, &'a ObligationsInBody),
) -> Result<SerializedTree> {
  if !needs_search {
    return tls::take_tree().ok_or(anyhow!(
      "failed to find tree for obligation target {hash:?}"
    ));
  }

  let (data, _) = thunk();

  let res: Result<SerializedTree> = data
    .iter()
    .find_map(|(obligation, this_hash, infcx)| {
      if this_hash == hash {
        log::info!("Generating synthetic tree for obligation {:?}", obligation);
        Some(generate_tree(infcx, &obligation))
      } else {
        None
      }
    })
    .unwrap_or_else(|| bail!("could not find tree with full search"));

  res
}

fn generate_tree<'tcx>(
  infcx: &InferCtxt<'tcx>,
  obligation: &PredicateObligation<'tcx>,
) -> Result<SerializedTree> {
  let goal = Goal {
    predicate: obligation.predicate,
    param_env: obligation.param_env,
  };

  let Some(body_id) = BODY_ID.copied() else {
    bail!("missing body id");
  };

  let body_owner = infcx.tcx.hir().body_owner_def_id(body_id).to_def_id();
  serialize_proof_tree(goal, infcx, body_owner)
}

pub(in crate::analysis) fn build_obligations_in_body<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  typeck_results: &'tcx TypeckResults<'tcx>,
) -> (Forgettable<FullData<'tcx>>, ObligationsInBody) {
  let obligations = tls::take_obligations();
  let obligation_data = tls::unsafe_take_data();

  let obligation_data = ObligationQueriesInBody::new(obligation_data);
  let mut synthetic_data = SyntheticQueriesInBody::new();

  let ctx = ErrorAssemblyCtx {
    tcx,
    body_id,
    typeck_results,
    obligations: &obligations,
    obligation_data: &obligation_data,
  };
  let reported_errors = tls::take_reported_errors();
  let bins = hir::associate_obligations_nodes(&ctx);

  let oib = transform::transform(
    tcx,
    body_id,
    typeck_results,
    obligations,
    &obligation_data,
    &mut synthetic_data,
    &reported_errors,
    bins,
  );

  (
    Forgettable::new(FullData {
      obligations: obligation_data,
      synthetic: synthetic_data,
    }),
    oib,
  )
}
