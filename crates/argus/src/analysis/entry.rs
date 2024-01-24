//! Code that relates two pieces of data, or computes the
//! rleationships between large structures.
use std::sync::Arc;

use anyhow::{anyhow, Result};
use fluid_let::fluid_let;
use index_vec::IndexVec;
use rustc_data_structures::fx::FxHashMap as HashMap;
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
  analysis::{
    ambiguous, hir,
    tls::{self, FullObligationData, UODIdx},
    EvaluationResult, Provenance, OBLIGATION_TARGET,
  },
  ext::{EvaluationResultExt, InferCtxtExt, TyCtxtExt},
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

  // FIXME: we really need to figure out when to save the full data.
  let dataid = Some(tls::unsafe_store_data(infcx, obl, result));

  let hir = infcx.tcx.hir();
  let fdata = infcx.bless_fulfilled(ldef_id, obl, result);

  // find enclosing HIR Expression

  // If the obligation is a TraitRef, ie.e., `TY: TRAIT`,
  // - get the self type
  log::debug!("Searching for predicate {:?}", fdata.obligation.predicate);
  let body_id = hir.body_owned_by(ldef_id);
  let hir_id = hir::find_most_enclosing_node(
    &infcx.tcx,
    body_id,
    fdata.obligation.cause.span,
  )
  .unwrap_or_else(|| hir.body_owner(body_id));

  log::debug!(
    r#"
    Found enclosing expression
      for PRED: {:#?}
      in  EXPR: {}
    "#,
    fdata.obligation.predicate,
    hir.node_to_string(hir_id)
  );

  let o = infcx.erase_non_local_data(fdata);
  let o = Provenance {
    originating_expression: hir_id,
    full_data: dataid,
    it: o,
  };

  // TODO: HACK: this is a quick fix to get the trait errors working,
  // we should do this more incremental / smarter.
  // TODO: The serialized predicate is also off, but we don't use that currently
  // in the ide, so not an issue (yet).
  for (span, preds) in infcx.reported_trait_errors.borrow().iter() {
    for pred in preds.iter() {
      let hash: ObligationHash = infcx.predicate_hash(pred).into();
      let hir_id = hir::find_most_enclosing_node(&infcx.tcx, body_id, *span)
        .unwrap_or_else(|| hir.body_owner(body_id));
      let value = serialize_to_value(infcx, &PredWrapper(&*pred))
        .expect("failed to serialize predicate");
      tls::maybe_add_trait_error(*span, value, Provenance {
        originating_expression: hir_id,
        full_data: None,
        it: hash,
      })
    }
  }

  tls::store_obligation(ldef_id, o);
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

pub struct ErrorAssemblyCtx<'a, 'tcx: 'a> {
  pub tcx: TyCtxt<'tcx>,
  pub body_id: BodyId,
  pub typeck_results: &'tcx TypeckResults<'tcx>,
  pub obligations: Vec<Provenance<ObligationHash>>,
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
  let name = hir.opt_name(hir.body_owner(body_id));
  let body = hir.body(body_id);
  let body_range = CharRange::from_span(body.value.span, source_map)
    .expect("Couldn't get body range");

  let obligations = tls::take_obligations();
  let obligation_data = obligations
    .iter()
    .filter_map(|o| {
      let idx = o.full_data?;
      let v = tls::unsafe_take_data(idx)?;
      Some((idx, v))
    })
    .collect::<HashMap<_, _>>();
  let obligation_data = ObligationQueriesInBody(obligation_data);

  let obligations_hash_only = obligations
    .iter()
    .map(|obl| obl.map(|o| o.hash))
    .collect::<Vec<_>>();

  let bound_info = tls::take_trait_error_info();
  let mut ctx = ErrorAssemblyCtx {
    tcx,
    body_id,
    typeck_results,
    obligations: obligations_hash_only,
    obligation_data: &obligation_data,
  };

  let trait_errors = ctx.assemble_bound_errors(bound_info);
  let ambiguity_errors = ctx.assemble_ambiguous_errors();
  let obligations = obligations
    .into_iter()
    .map(|p| p.forget())
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
