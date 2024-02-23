//! Proof tree types sent to the Argus frontend.

pub mod ext;
pub(super) mod serialize;
pub mod topology;

use std::collections::HashSet;

use index_vec::IndexVec;
use rustc_infer::infer::InferCtxt;
use rustc_middle::ty;
use serde::Serialize;
pub use topology::*;
#[cfg(feature = "testing")]
use ts_rs::TS;

use crate::{
  ext::{InferCtxtExt, PredicateExt},
  serialize::{safe::GoalPredicateDef, serialize_to_value},
  types::{
    intermediate::{EvaluationResult, EvaluationResultDef},
    ImplHeader, ObligationNecessity,
  },
};

crate::define_idx! {
  usize,
  ProofNodeIdx,
  GoalIdx
}

// FIXME: Nodes shouldn't be PartialEq, or Eq. They are currently
// so we can "detect cycles" by doing a raw comparison of the nodes.
// Of course, this isn't robust and should be removed ASAP.
//
// Same goes for Candidates and Goals.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum Node {
  Result(
    #[serde(with = "EvaluationResultDef")]
    #[cfg_attr(feature = "testing", ts(type = "EvaluationResult"))]
    EvaluationResult,
  ),
  Candidate(Candidate),
  Goal(
    GoalIdx,
    #[serde(with = "EvaluationResultDef")]
    #[cfg_attr(feature = "testing", ts(type = "EvaluationResult"))]
    EvaluationResult,
  ),
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct Goal {}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum Candidate {
  Impl(
    #[cfg_attr(feature = "testing", ts(type = "ImplHeader"))] serde_json::Value,
  ),
  ParamEnv(usize),
  // TODO remove variant once everything is structured
  Any(String),
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct GoalData {
  #[cfg_attr(feature = "testing", ts(type = "GoalPredicate"))]
  value: serde_json::Value,

  necessity: ObligationNecessity,
  num_vars: usize,
  is_lhs_ty_var: bool,

  #[cfg(debug_assertions)]
  #[cfg_attr(feature = "testing", ts(type = "string | undefined"))]
  debug_comparison: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct SerializedTree {
  pub root: ProofNodeIdx,

  #[cfg_attr(feature = "testing", ts(type = "Node[]"))]
  pub nodes: IndexVec<ProofNodeIdx, Node>,
  #[cfg_attr(feature = "testing", ts(type = "GoalData[]"))]
  pub goals: IndexVec<GoalIdx, GoalData>,

  pub topology: TreeTopology,
  pub error_leaves: Vec<ProofNodeIdx>,
  pub unnecessary_roots: HashSet<ProofNodeIdx>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cycle: Option<ProofCycle>,
}

#[derive(Serialize, Debug, Clone)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ProofCycle(Vec<ProofNodeIdx>);

// ----------------------------------------
// impls

impl Candidate {
  fn new_impl_header<'tcx>(
    infcx: &InferCtxt<'tcx>,
    impl_: &ImplHeader<'tcx>,
  ) -> Self {
    let impl_ =
      serialize_to_value(infcx, impl_).expect("couldn't serialize impl header");

    Self::Impl(impl_)
  }

  // TODO: we should pass the ParamEnv here for certainty.
  fn new_param_env(idx: usize) -> Self {
    Self::ParamEnv(idx)
  }
}

impl From<&'static str> for Candidate {
  fn from(value: &'static str) -> Self {
    value.to_string().into()
  }
}

impl From<String> for Candidate {
  fn from(value: String) -> Self {
    Candidate::Any(value)
  }
}
