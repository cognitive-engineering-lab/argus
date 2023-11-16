//! ProofTree analysis.

use std::ops::ControlFlow;

use rustc_hir::{BodyId, FnSig};
use rustc_hir_analysis::astconv::AstConv;
use rustc_hir_typeck::{FnCtxt, Inherited};
use rustc_infer::infer::InferCtxt;
use rustc_middle::ty::{TyCtxt, Predicate};

use rustc_trait_selection::solve::inspect::{ProofTreeVisitor, InspectGoal, ProofTreeInferCtxtExt};
use rustc_trait_selection::traits::{ObligationCtxt, FulfillmentError};
use rustc_trait_selection::traits::solve::{Goal, QueryInput};
use rustc_type_ir::Canonical;

use rustc_utils::mir::body::BodyExt;

use serde::Serialize;
use index_vec::IndexVec;

use crate::proof_tree::{ProofNodeIdx, SerializedTree};
use crate::proof_tree::ext::{CanonicalGoal, InspectGoalExt};
use crate::proof_tree::serialize::serialize_proof_tree;
use crate::proof_tree::topology::TreeTopology;
// use crate::proof_tree::visitor::{ProofTreeInferCtxtExt, ProofTreeVisitor, InspectGoal};

pub fn trees_in_body(tcx: TyCtxt, body_id: BodyId) -> Vec<SerializedTree> {
  let hir = tcx.hir();
  let def_id = hir.body_owner_def_id(body_id);
  let hir_id = hir.local_def_id_to_hir_id(def_id);
  let body = hir.body(body_id);

  if let Some(FnSig { decl, .. }) = hir.fn_sig_by_hir_id(hir_id) {

    let param_env = tcx.param_env(def_id);

    let inh = Inherited::new(tcx, def_id);
    let mut fcx = FnCtxt::new(&inh, param_env, def_id);
    let fn_sig = tcx.fn_sig(def_id).instantiate_identity();
    let fn_sig = tcx.liberate_late_bound_regions(def_id.to_def_id(), fn_sig);
    let fn_sig = fcx.normalize(body.value.span, fn_sig);

    let _ = rustc_hir_typeck::check_fn(
      &mut fcx,
      fn_sig,
      decl,
      def_id,
      body,
      None,
      tcx.features().unsized_fn_params,
    );

    let errors = fcx.fulfillment_errors.borrow();

    errors.iter().flat_map(|error| {
      serialize_error_tree(error, &fcx)
    }).collect::<Vec<_>>()
  } else {
      Vec::default()
  }
}

fn serialize_error_tree<'tcx>(error: &FulfillmentError<'tcx>, fcx: &FnCtxt<'_, 'tcx>) -> Option<SerializedTree> {
  let o = &error.root_obligation;
  let goal = Goal { predicate: o.predicate, param_env: o.param_env };
  let def_id = fcx.item_def_id();
  let infcx = fcx.infcx().expect("`FnCtxt` missing a `InferCtxt`.");

  serialize_proof_tree(goal, infcx, def_id)
}
