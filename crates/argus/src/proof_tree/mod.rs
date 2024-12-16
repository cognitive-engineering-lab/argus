//! Proof tree types sent to the Argus frontend.

mod format;
mod interners;
pub(super) mod serialize;
pub mod topology;

use std::collections::HashMap;

use argus_ext::ty::PredicateExt;
use argus_ser::{self as ser, interner::TyIdx};
use index_vec::IndexVec;
use rustc_infer::infer::InferCtxt;
use rustc_middle::ty;
use serde::Serialize;
use serde_json as json;
pub use topology::*;
#[cfg(feature = "testing")]
use ts_rs::TS;

use crate::{
  aadebug, tls,
  types::{
    intermediate::{EvaluationResult, EvaluationResultDef},
    ObligationNecessity,
  },
};

ser::define_idx! {
  u32,
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
  value: json::Value,

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
  Impl {
    #[cfg_attr(feature = "testing", ts(type = "ImplHeader"))]
    hd: json::Value,
    is_user_visible: bool,
  },
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

  #[cfg_attr(feature = "testing", ts(type = "TyVal[]"))]
  pub tys: IndexVec<TyIdx, json::Value>,

  pub projection_values: HashMap<TyIdx, TyIdx>,

  pub all_impl_candidates: HashMap<ProofNodeIdx, Implementors>,

  pub topology: TreeTopology,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub cycle: Option<ProofCycle>,

  pub analysis: aadebug::AnalysisResults,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct Implementors {
  #[cfg_attr(feature = "testing", ts(type = "TraitRefPrintOnlyTraitPath"))]
  pub _trait: json::Value,
  pub impls: Vec<CandidateIdx>,
  pub inductive_impls: Vec<CandidateIdx>,
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
    impl_: &ser::ImplHeader<'tcx>,
    is_user_visible: bool,
  ) -> Self {
    let impl_ = tls::unsafe_access_interner(|ty_interner| {
      ser::to_value_expect(infcx, ty_interner, impl_)
    });

    Self::Impl {
      hd: impl_,
      is_user_visible,
    }
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
