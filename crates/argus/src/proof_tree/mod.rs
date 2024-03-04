//! Proof tree types sent to the Argus frontend.

pub mod ext;
pub(self) mod interners;
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
  GoalIdx,
  CandidateIdx,
  ResultIdx
}

#[derive(Serialize, Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum Node {
  Goal(GoalIdx),
  Candidate(CandidateIdx),
  Result(ResultIdx),
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
  /// Is one of the main components a type variable?
  ///
  /// This would be a trait clause like `_: TRAIT` or a projection where `PROJ == _`.
  is_main_tv: bool,
  result: ResultIdx,

  #[cfg(debug_assertions)]
  #[cfg_attr(feature = "testing", ts(type = "string | undefined"))]
  debug_comparison: String,
}

#[derive(Serialize, Clone, Debug)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum CandidateData {
  Impl(
    #[cfg_attr(feature = "testing", ts(type = "ImplHeader"))] serde_json::Value,
  ),
  ParamEnv(usize),
  // TODO remove variant once everything is structured
  Any(String),
}

#[derive(Serialize, Clone, Debug)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ResultData(
  #[serde(with = "EvaluationResultDef")]
  #[cfg_attr(feature = "testing", ts(type = "EvaluationResult"))]
  EvaluationResult,
);

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

  #[cfg_attr(feature = "testing", ts(type = "CandidateData[]"))]
  pub candidates: IndexVec<CandidateIdx, CandidateData>,

  #[cfg_attr(feature = "testing", ts(type = "ResultData[]"))]
  pub results: IndexVec<ResultIdx, ResultData>,

  pub topology: TreeTopology,
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

impl CandidateData {
  fn new_impl_header<'tcx>(
    infcx: &InferCtxt<'tcx>,
    impl_: &ImplHeader<'tcx>,
  ) -> Self {
    let impl_ =
      serialize_to_value(infcx, impl_).expect("couldn't serialize impl header");
    Self::Impl(impl_)
  }
}

impl From<&'static str> for CandidateData {
  fn from(value: &'static str) -> Self {
    value.to_string().into()
  }
}

impl From<String> for CandidateData {
  fn from(value: String) -> Self {
    Self::Any(value)
  }
}
