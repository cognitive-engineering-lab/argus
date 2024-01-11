//! ProofTree analysis.
use anyhow::{anyhow, Result};
use fluid_let::fluid_let;
use rustc_data_structures::stable_hasher::Hash64;
use rustc_hir::BodyId;
use rustc_hir_analysis::astconv::AstConv;
use rustc_hir_typeck::{inspect_typeck, FnCtxt};
use rustc_middle::ty::{self, TyCtxt};
use rustc_trait_selection::traits::{solve::Goal, FulfillmentError};
use rustc_utils::source_map::range::CharRange;

use crate::{
  ext::{FnCtxtExt as ArgusFnCtxtExt, InferCtxtExt},
  proof_tree::{serialize::serialize_proof_tree, SerializedTree},
  serialize::serialize_to_value,
  types::{ObligationsInBody, Target},
};

fluid_let!(pub static OBLIGATION_TARGET: Target);

pub(crate) type FulfillmentData<'tcx> = (Hash64, FulfillmentError<'tcx>);

pub fn obligations<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
) -> Result<serde_json::Value> {
  let hir = tcx.hir();
  let local_def_id = hir.body_owner_def_id(body_id);

  log::info!("Getting obligations in body {}", {
    let owner = hir.body_owner(body_id);
    hir
      .opt_name(owner)
      .map(|s| s.to_string())
      .unwrap_or("<anon body>".to_string())
  });

  let mut result = Err(anyhow!("Hir Typeck never called inspect fn."));

  inspect_typeck(tcx, local_def_id, |fncx| {
    let Some(infcx) = fncx.infcx() else {
      return;
    };

    let obligations = fncx.get_obligations(local_def_id);
    let body = hir.body(body_id);
    let source_map = tcx.sess.source_map();
    let body_range = CharRange::from_span(body.value.span, source_map)
      .expect("Couldn't get body range");

    let trait_errors = infcx.build_trait_errors(&obligations);

    let obligation_in_body = ObligationsInBody {
      name: hir.opt_name(body_id.hir_id),
      range: body_range,
      ambiguity_errors: vec![],
      trait_errors,
      obligations,
    };

    result =
      serialize_to_value(&obligation_in_body, infcx).map_err(|e| anyhow!(e));
  });

  result
}

// NOTE: tree is only invoked for *a single* tree, it must be found
// within the `body_id` and the appropriate `OBLIGATION_TARGET` (i.e., stable hash).
pub fn tree<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
) -> Result<serde_json::Value> {
  OBLIGATION_TARGET.get(|target| {
    let target = target.unwrap();

    let hir = tcx.hir();
    let local_def_id = hir.body_owner_def_id(body_id);
    let mut result =
      Err(anyhow!("Couldn't find proof tree with hash {:?}", target));

    inspect_typeck(tcx, local_def_id, |fncx| {
      let Some(infcx) = fncx.infcx() else {
        return;
      };

      let errors = fncx.get_fulfillment_errors(local_def_id);
      let found_tree = errors.iter().find_map(|(hash, error)| {
        if hash.as_u64() != *target.hash {
          return None;
        }

        let goal = Goal {
          predicate: error.root_obligation.predicate,
          param_env: error.root_obligation.param_env,
        };
        let serial_tree = serialize_tree(goal, fncx)?;
        serialize_to_value(&serial_tree, infcx).ok()
      });

      result = found_tree
        .ok_or(anyhow!("Couldn't find proof tree with hash {:?}", target));
    });

    result
  })
}

fn serialize_tree<'tcx>(
  goal: Goal<'tcx, ty::Predicate<'tcx>>,
  fcx: &FnCtxt<'_, 'tcx>,
) -> Option<SerializedTree<'tcx>> {
  let def_id = fcx.item_def_id();
  let infcx = fcx.infcx().expect("`FnCtxt` missing a `InferCtxt`.");

  serialize_proof_tree(goal, infcx, def_id)
}
