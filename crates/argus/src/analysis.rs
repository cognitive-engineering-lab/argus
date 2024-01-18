//! ProofTree analysis.
use std::cell::RefCell;

use anyhow::{anyhow, Result};
use fluid_let::fluid_let;
use rustc_data_structures::stable_hasher::Hash64;
use rustc_hir::BodyId;
use rustc_hir::def_id::LocalDefId;
use rustc_hir_analysis::astconv::AstConv;
use rustc_hir_typeck::{
  inspect_typeck,
  FnCtxt
};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use rustc_middle::traits::{solve::Certainty, query::NoSolution};
use rustc_trait_selection::traits::{solve::Goal, FulfillmentError};
use rustc_utils::source_map::range::CharRange;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};

pub(crate) use crate::types::intermediate::{EvaluationResult, FulfillmentData};
use crate::{
  ext::InferCtxtExt,
  proof_tree::{serialize::serialize_proof_tree, SerializedTree},
  serialize::serialize_to_value,
  types::{ObligationsInBody, Target, Obligation, ObligationKind},
  tls,
};

fluid_let! {
  pub static OBLIGATION_TARGET: Target;

  static INSPECTING: bool;
}

// ---

macro_rules! guard_inspection {
  () => {{
    if INSPECTING.copied().unwrap_or(false) {
      return;
    }
  }}
}

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

  // Typecks the current body and invokes `process_obligation` for each
  // obligation solved for. Our information accumulates in thread local.
  inspect_typeck(tcx, local_def_id, process_obligation);

  let source_map = tcx.sess.source_map();
  let name = hir.opt_name(hir.body_owner(body_id));
  let body = hir.body(body_id);
  let body_range = CharRange::from_span(body.value.span, source_map)
    .expect("Couldn't get body range");

  let obligations = tls::take_obligations();
  let ambiguity_errors = tls::get_ambiguity_errors();
  let trait_errors = tls::get_trait_errors();

  let obligations_in_body = ObligationsInBody {
    name,
    range: body_range,
    ambiguity_errors,
    trait_errors,
    obligations,
  };

  serde_json::to_value(&obligations_in_body)
    .map_err(|e| anyhow!(e))
}

// NOTE: tree is only invoked for *a single* tree, it must be found
// within the `body_id` and the appropriate `OBLIGATION_TARGET` (i.e., stable hash).
pub fn tree<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
) -> Result<serde_json::Value> {
  let local_def_id = tcx.hir().body_owner_def_id(body_id);
  inspect_typeck(tcx, local_def_id, process_obligation_for_tree);
  tls::take_tree().ok_or_else(|| {
    OBLIGATION_TARGET.get(|target| {
      anyhow!("failed to locate proof tree with target {:?}", target)
    })
  })
}

// --------------------------------
// Rustc inspection points

fn process_obligation<'tcx>(infcx: &InferCtxt<'tcx>, obl: &PredicateObligation<'tcx>, result: EvaluationResult) {
  guard_inspection! {}

  let Some(ldef_id) = infcx.body_id() else {
    todo!()
  };

  let fdata = infcx.bless_fulfilled(ldef_id, obl, result);
  let o = infcx.erase_non_local_data(fdata);

  tls::push_obligation(ldef_id, o);
}

fn process_obligation_for_tree<'tcx>(infcx: &InferCtxt<'tcx>, obl: &PredicateObligation<'tcx>, result: EvaluationResult) {
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
        let serial_tree =
          serialize_proof_tree(goal, infcx, item_def_id)?;

        serialize_to_value(infcx, &serial_tree).ok()
      };

      if let Some(stree)  = inner() {
        tls::store_tree(stree);
      }
    })
  })
}
