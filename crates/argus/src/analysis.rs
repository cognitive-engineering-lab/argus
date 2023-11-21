//! ProofTree analysis.

use std::ops::ControlFlow;
use std::cell::Cell;

use rustc_hir::{BodyId, FnSig};
use rustc_hir_analysis::astconv::AstConv;
use rustc_hir_typeck::{inspect_typeck, FnCtxt, Inherited};
use rustc_infer::infer::InferCtxt;
use rustc_infer::traits::FulfilledObligation;
use rustc_middle::ty::{TyCtxt, Predicate};
use rustc_trait_selection::solve::inspect::{ProofTreeVisitor, InspectGoal, ProofTreeInferCtxtExt};
use rustc_trait_selection::traits::{ObligationCtxt, FulfillmentError};
use rustc_trait_selection::traits::solve::{Goal, QueryInput};
use rustc_type_ir::Canonical;
use rustc_span::{Span, DUMMY_SP};
use rustc_utils::{
  source_map::range::CharRange,
  mir::body::BodyExt,
};

use serde::Serialize;
use ts_rs::TS;
use index_vec::IndexVec;
use anyhow::{Result, Context, bail};
use fluid_let::fluid_let;

use crate::proof_tree::{ProofNodeIdx, SerializedTree, Obligation};
use crate::proof_tree::pretty::{PrettyPrintExt, PrettyCandidateExt, PrettyResultExt};
use crate::proof_tree::serialize::serialize_proof_tree;
use crate::proof_tree::topology::TreeTopology;

fluid_let!(pub static OBLIGATION_TARGET_SPAN: Span);

pub fn obligations(tcx: TyCtxt, body_id: BodyId) -> Result<Vec<Obligation>> {
  use FulfilledObligation::*;

  let hir = tcx.hir();
  let local_def_id = hir.body_owner_def_id(body_id);
  let def_id = local_def_id.to_def_id();

  log::info!("Getting obligations");

  let mut result = Vec::new();

  inspect_typeck(tcx, local_def_id, |fncx| {
    if let Some(infcx) = fncx.infcx() {
      let source_map = infcx.tcx.sess.source_map();

      let fulfilled_obligations = infcx.fulfilled_obligations.borrow();

      result.extend(
        fulfilled_obligations.iter().filter_map(|obl| {
          match obl {
            Success(obligation) => {
              None
              // let range = CharRange::from_span(obligation.cause.span, source_map).unwrap();
              // Some(Obligation::Success {
              //   range,
              //   data: obligation.predicate.pretty(infcx, def_id)
              // })
            },
            Failure(error) => {
              let range = CharRange::from_span(error.root_obligation.cause.span, source_map).unwrap();
              Some(Obligation::Failure {
                range,
                data: error.obligation.predicate.pretty(infcx, def_id)
              })
            },
          }
        })
      )
    }
  });

  Ok(result)
}

pub fn tree(tcx: TyCtxt, body_id: BodyId) -> Result<Vec<SerializedTree>> {
  use FulfilledObligation::*;

  let target_span = OBLIGATION_TARGET_SPAN.copied().unwrap_or(DUMMY_SP);

  let hir = tcx.hir();
  let local_def_id = hir.body_owner_def_id(body_id);
  let def_id = local_def_id.to_def_id();

  log::info!("Getting obligations");

  let mut result = Vec::new();

  inspect_typeck(tcx, local_def_id, |fncx| {
    if let Some(infcx) = fncx.infcx() {
      let fulfilled_obligations = infcx.fulfilled_obligations.borrow();

      result.extend(
        fulfilled_obligations.iter().filter_map(|obl| {
          match obl {
            Success(_) => None,
            Failure(error) => {
              let here_span = error.root_obligation.cause.span;

              if !here_span.overlaps(target_span) {
                return None;
              }

              serialize_error_tree(&error, fncx)
            },
          }
        })
      )
    }
  });

  Ok(result)
}

fn serialize_error_tree<'tcx>(error: &FulfillmentError<'tcx>, fcx: &FnCtxt<'_, 'tcx>) -> Option<SerializedTree> {
  let o = &error.root_obligation;
  let goal = Goal { predicate: o.predicate, param_env: o.param_env };
  let def_id = fcx.item_def_id();
  let infcx = fcx.infcx().expect("`FnCtxt` missing a `InferCtxt`.");

  serialize_proof_tree(goal, infcx, def_id)
}
