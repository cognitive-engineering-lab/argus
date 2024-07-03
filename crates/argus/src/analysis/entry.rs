//! Code that relates two pieces of data, or computes the
//! rleationships between large structures.

use anyhow::{anyhow, bail, Result};
use argus_ext::ty::EvaluationResultExt;
use fluid_let::fluid_let;
use rustc_hir::BodyId;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::ty::{TyCtxt, TypeckResults};
use rustc_trait_selection::traits::solve::Goal;

use crate::{
  analysis::{
    hir, transform, EvaluationResult, INCLUDE_SUCCESSES, OBLIGATION_TARGET,
  },
  ext::InferCtxtExt,
  proof_tree::{serialize::try_serialize, SerializedTree},
  tls,
  types::{
    intermediate::{ErrorAssemblyCtx, Forgettable, FullData},
    ObligationHash, ObligationNecessity, ObligationsInBody,
  },
};

fluid_let! {
  pub static INSPECTING: bool;
  pub static BODY_ID: BodyId;
}

macro_rules! guard_inspection {
  () => {
    guard_inspection! { return; };
  };
  ($($t:tt)+) => {
    if INSPECTING.copied().unwrap_or(false) {
      $($t)+
    }
    fluid_let::fluid_set!(INSPECTING, true);
  };
}

// --------------------------------
// Rustc inspection points

pub fn process_obligation<'tcx>(
  infcx: &InferCtxt<'tcx>,
  obl: &PredicateObligation<'tcx>,
  result: EvaluationResult,
) {
  guard_inspection! {}

  let Some(body_id) = BODY_ID.copied() else {
    return;
  };

  log::trace!("RECV OBLIGATION {result:?} {obl:?}");

  // Use this to get rid of any resolved inference variables,
  // these could have been resolved while trying to solve the obligation
  // and we want to present it as such to the user.
  let obl = &infcx.resolve_vars_if_possible(obl.clone());

  // HACK: Remove ambiguous obligations if a "stronger" result was found and
  // the predicate implies the  previous. This is necessary because we
  // can't (currently) distinguish between a subsequent solving attempt
  // of a previous obligation.
  if result.is_yes() || result.is_no() {
    tls::drain_implied_ambiguities(infcx, obl);
  }

  if !INCLUDE_SUCCESSES.copied().unwrap_or(false) && result.is_yes() {
    log::debug!("Skipping successful obligation {obl:?}");
    return;
  }

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

    // Must go after the synthetic check.
    guard_inspection! {}

    // Use this to get rid of any resolved inference variables,
    // these could have been resolved while trying to solve the obligation
    // and we want to present it as such to the user.
    let obl = &infcx.resolve_vars_if_possible(obl.clone());

    let fdata = infcx.bless_fulfilled(obl, result);

    if fdata.hash != target.hash {
      return;
    }

    match generate_tree(infcx, obl, fdata.result) {
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
) -> ObligationsInBody {
  log::trace!("build_obligations_output {body_id:?}");
  let (_, oib) = build_obligations_in_body(tcx, body_id, typeck_results);
  oib
}

pub fn build_tree_output<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  typeck_results: &'tcx TypeckResults<'tcx>,
) -> Result<SerializedTree> {
  log::trace!("build_tree_output {body_id:?}");
  OBLIGATION_TARGET.get(|target| {
    let target = target.ok_or(anyhow!("missing target"))?;
    let (data, oib) = build_obligations_in_body(tcx, body_id, typeck_results);
    pick_tree(target.hash, || (&*data, &oib))
  })
}

pub(crate) fn pick_tree<'a, 'tcx: 'a>(
  hash: ObligationHash,
  thunk: impl FnOnce() -> (&'a FullData<'tcx>, &'a ObligationsInBody),
) -> Result<SerializedTree> {
  log::trace!("pick_tree {hash:?}");

  guard_inspection! {
    anyhow::bail!("already inspecting tree")
  }

  if let Some(tree) = tls::take_tree() {
    return Ok(tree);
  }

  let (data, _) = thunk();

  let res: Result<SerializedTree> = data
    .iter()
    .find_map(|fdata| {
      if fdata.hash == hash {
        log::info!("Generating tree for obligation {:?}", fdata.obligation);
        Some(generate_tree(&fdata.infcx, &fdata.obligation, fdata.result))
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
  result: EvaluationResult,
) -> Result<SerializedTree> {
  log::trace!("generate_tree {obligation:?} {result:?}");

  let goal = Goal {
    predicate: obligation.predicate,
    param_env: obligation.param_env,
  };

  let Some(body_id) = BODY_ID.copied() else {
    bail!("missing body id");
  };

  let body_owner = infcx.tcx.hir().body_owner_def_id(body_id).to_def_id();
  try_serialize(goal, result, obligation.cause.span, infcx, body_owner)
}

pub(in crate::analysis) fn build_obligations_in_body<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  typeck_results: &'tcx TypeckResults<'tcx>,
) -> (Forgettable<FullData<'tcx>>, ObligationsInBody) {
  let obligations = tls::take_obligations();
  let obligation_data = tls::unsafe_take_data();

  let obligation_data = FullData::new(obligation_data);

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
    &reported_errors,
    bins,
  );

  (Forgettable::new(obligation_data), oib)
}
