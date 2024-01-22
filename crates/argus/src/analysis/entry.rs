//! Code that relates two pieces of data, or computes the
//! rleationships between large structures.
use std::sync::Arc;

use anyhow::{anyhow, Result};
use fluid_let::fluid_let;
use rustc_data_structures::fx::FxIndexMap as IMap;
use rustc_hir::{
  def_id::LocalDefId, hir_id::HirId, intravisit::Visitor as HirVisitor, BodyId,
};
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::ty::{Predicate, TyCtxt, TypeckResults};
use rustc_span::Span;
use rustc_trait_selection::traits::solve::Goal;
use rustc_utils::source_map::range::CharRange;
use serde::Serialize;

use crate::{
  analysis::{ambiguous, tls, EvaluationResult, Provenance, OBLIGATION_TARGET},
  ext::{InferCtxtExt, TyCtxtExt},
  proof_tree::serialize::serialize_proof_tree,
  serialize::{serialize_to_value, ty::PredicateDef},
  types::{
    AmbiguityError, Obligation, ObligationHash, ObligationsInBody, Target,
    TraitError,
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

  let Some(ldef_id) = infcx.body_id() else {
    todo!()
  };

  let hir = infcx.tcx.hir();
  let fdata = infcx.bless_fulfilled(ldef_id, obl, result);

  // find enclosing HIR Expression

  // If the obligation is a TraitRef, ie.e., `TY: TRAIT`,
  // - get the self type
  let body_id = hir.body_owned_by(ldef_id);
  // let Some(expr) = infcx
  //   .tcx
  //   .find_expr_by_span(body_id, fdata.obligation.cause.span)
  // else {
  //   log::warn!(
  //     "Couldn't find expression for obligation {:#?}",
  //     fdata.obligation
  //   );
  //   return;
  // };

  // log::debug!(
  //   "Found enclosing expression for {:#?}\n\tEXPR: {}",
  //   fdata.obligation,
  //   hir.node_to_string(expr.hir_id)
  // );

  // - find all types it derefernces to

  // let o = infcx.erase_non_local_data(fdata);
  // let o = todo!();

  // TODO: HACK: this is a quick fix to get the trait errors working,
  // we should do this more incremental / smarter.
  // The serialized predicate is also off, but we don't use that currently
  // in the ide, so not an issue (yet).
  let reported_trait_errors = infcx.reported_trait_errors.borrow();
  for (span, preds) in reported_trait_errors.iter() {
    log::debug!("Inserting trait errors {:#?}", preds);

    for pred in preds.iter() {
      let hash: ObligationHash = infcx.predicate_hash(pred).into();

      // TODO: this isn't well thought hout.
      let hir_id = match infcx.tcx.find_expr_by_span(body_id, *span) {
        Some(expr) => expr.hir_id,
        None => hir.body_owner(body_id),
      };

      let value = serialize_to_value(infcx, &"TODO")
        .expect("failed to serialize predicate");

      tls::maybe_add_trait_error(*span, value, Provenance {
        originating_expression: hir_id,
        it: hash,
      })
    }
  }

  // tls::store_obligation(ldef_id, o);
}

// --------------------------------

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

/// Retrieve *all* obligations processed from rustc.
pub fn build_obligations_output<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
  typeck_results: &TypeckResults<'tcx>,
) -> Result<serde_json::Value> {
  let hir = tcx.hir();
  let source_map = tcx.sess.source_map();
  let name = hir.opt_name(hir.body_owner(body_id));
  let body = hir.body(body_id);
  let body_range = CharRange::from_span(body.value.span, source_map)
    .expect("Couldn't get body range");

  let obligations = tls::take_obligations()
    .into_iter()
    .map(|(_, p)| p.forget())
    .collect::<Vec<_>>();

  let trait_errors = tls::assemble_trait_errors(&tcx);
  let ambiguity_errors =
    ambiguous::get_ambiguous_trait_method_exprs(&tcx, typeck_results)
      .into_iter()
      .map(|data| {
        let span = hir.span(data.expr.hir_id);
        let range = CharRange::from_span(span, source_map)
          .expect("Couldn't get ambiguous method range");

        AmbiguityError { range }
      })
      .collect::<Vec<_>>();

  let oib = ObligationsInBody {
    name,
    range: body_range,
    ambiguity_errors,
    trait_errors,
    obligations,
  };

  serde_json::to_value(&oib).map_err(|e| anyhow!(e))
}

// --------------------------------

pub fn build_tree_output<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
) -> Option<serde_json::Value> {
  tls::take_tree()
}
