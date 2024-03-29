pub(crate) mod entry;
mod hir;
mod tls;
mod transform;

use anyhow::Result;
use fluid_let::fluid_let;
use rustc_hir::BodyId;
use rustc_middle::ty::TyCtxt;
pub(crate) use tls::{FullObligationData, SynIdx, UODIdx};

pub(crate) use crate::types::intermediate::{
  EvaluationResult, FulfillmentData,
};
#[cfg(feature = "testing")]
use crate::types::intermediate::{Forgettable, FullData};
use crate::{
  ext::TyCtxtExt,
  proof_tree::SerializedTree,
  types::{ObligationsInBody, Target},
};

fluid_let! {
  pub static OBLIGATION_TARGET: Target;
  pub static INCLUDE_SUCCESSES: bool;
}

pub fn obligations<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
) -> Result<ObligationsInBody> {
  rustc_middle::ty::print::with_no_trimmed_paths! {{
    log::info!("Typeck'ing body {body_id:?}");

    let typeck_results = tcx.inspect_typeck(body_id, entry::process_obligation);

    // Construct the output from the stored data.
    entry::build_obligations_output(tcx, body_id, typeck_results)
  }}
}

// NOTE: tree is only invoked for *a single* tree, it must be found
// within the `body_id` and the appropriate `OBLIGATION_TARGET` (i.e., stable hash).
pub fn tree<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
) -> Result<SerializedTree> {
  rustc_middle::ty::print::with_no_trimmed_paths! {{
    let typeck_results =
      tcx.inspect_typeck(body_id, entry::process_obligation_for_tree);

    entry::build_tree_output(tcx, body_id, typeck_results)
  }}
}

#[cfg(feature = "testing")]
pub(crate) fn body_data<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
) -> Result<(Forgettable<FullData<'tcx>>, ObligationsInBody)> {
  let typeck_results = tcx.inspect_typeck(body_id, entry::process_obligation);
  Ok(entry::build_obligations_in_body(
    tcx,
    body_id,
    typeck_results,
  ))
}
