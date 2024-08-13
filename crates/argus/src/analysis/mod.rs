pub(crate) mod entry;
mod hir;
mod transform;

use std::collections::HashMap;

use anyhow::Result;
use argus_ext::ty::TyCtxtExt;
use fluid_let::fluid_let;
use rustc_hir::BodyId;
use rustc_middle::ty::TyCtxt;

pub(crate) use crate::types::intermediate::{
  EvaluationResult, FulfillmentData,
};
use crate::{
  proof_tree::SerializedTree,
  types::{
    intermediate::{Forgettable, FullData},
    BodyBundle, ObligationNecessity, ObligationsInBody, Target,
  },
};

fluid_let! {
  pub static OBLIGATION_TARGET: Target;
  pub static INCLUDE_SUCCESSES: bool;
}

/// Check the workspace as rustc would, but with the new solver
///
/// Returns `true` if the body is tainted by errors and wouldn't type check.
pub fn check(tcx: TyCtxt, body_id: BodyId) -> Result<bool> {
  fluid_let::fluid_set!(entry::BODY_ID, body_id);
  let typeck_results = tcx.inspect_typeck(body_id, |_, _, _| {});
  log::info!(
    "check {body_id:?} tainted? {:?}",
    typeck_results.tainted_by_errors
  );
  Ok(typeck_results.tainted_by_errors.is_some())
}

/// Generate the set of evaluated obligations within a single body.
pub fn obligations(tcx: TyCtxt, body_id: BodyId) -> Result<ObligationsInBody> {
  fluid_let::fluid_set!(entry::BODY_ID, body_id);

  let typeck_results = tcx.inspect_typeck(body_id, entry::process_obligation);

  // Construct the output from the stored data.
  Ok(entry::build_obligations_output(
    tcx,
    body_id,
    typeck_results,
  ))
}

/// Generate a *single* proof-tree for a target obligation within a body. See
/// `OBLIGATION_TARGET` for target data.
pub fn tree(tcx: TyCtxt, body_id: BodyId) -> Result<SerializedTree> {
  fluid_let::fluid_set!(entry::BODY_ID, body_id);

  log::trace!("tree {body_id:?}");

  let typeck_results =
    // tcx.inspect_typeck(body_id, entry::process_obligation_for_tree);
    tcx.inspect_typeck(body_id, entry::process_obligation);

  entry::build_tree_output(tcx, body_id, typeck_results)
}

/// Analyze all bodies and pre-generate the necessary proof trees for self-contained output.
///
/// NOTE: this requires quite a bit of memory as everything is generated eagerly, favor
/// using a combination of `obligation` and `tree` analyses for a reduced memory footprint.
pub fn bundle(tcx: TyCtxt, body_id: BodyId) -> Result<BodyBundle> {
  fluid_let::fluid_set!(entry::BODY_ID, body_id);

  log::trace!("bundle {body_id:?}");

  let (full_data, obligations_in_body) = body_data(tcx, body_id);
  let t = (&*full_data, &obligations_in_body);
  let thunk = || t;

  let mut trees = HashMap::new();
  for obl in &t.1.obligations {
    if obl.necessity == ObligationNecessity::Yes
      || (obl.necessity == ObligationNecessity::OnError && obl.result.is_err())
    {
      if let Ok(stree) = entry::pick_tree(obl.hash, thunk) {
        trees.insert(obl.hash, stree);
      }
    }
  }

  let filename = tcx
    .body_filename(body_id)
    .prefer_local()
    .to_string_lossy()
    .to_string();

  Ok(BodyBundle {
    filename,
    body: obligations_in_body,
    trees,
  })
}

pub(crate) fn body_data(
  tcx: TyCtxt,
  body_id: BodyId,
) -> (Forgettable<FullData>, ObligationsInBody) {
  let typeck_results = tcx.inspect_typeck(body_id, entry::process_obligation);
  entry::build_obligations_in_body(tcx, body_id, typeck_results)
}
