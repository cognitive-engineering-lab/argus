//! Proof tree types sent to the Argus frontend.

mod format;
mod hash_map;
mod interners;
pub(super) mod serialize;
pub mod topology;

use argus_ext::ty::PredicateExt;
use argus_ser::{self as ser, interner::TyIdx};
use hash_map::HashMap;
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
    ObligationNecessity,
    intermediate::{EvaluationResult, EvaluationResultDef},
  },
};

// NOTE all indices need to be exported
// manually with ts-rs, see end of file.
ser::define_idx! {
  u32,
  GoalIdx,
  ImplementorsIdx,
  CandidateIdx,
  ResultIdx
}

#[derive(Serialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ProofNode(u32);

/// [`ProofNodeUnpacked`] should only be used temporarily (e.g., not saved to
/// the heap). In comparison to a [`ProofNode`], it is not as versatile because
/// its JS representation cannot be used as a map key, and it is 8 bytes instead
/// of 4.
#[derive(Serialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum ProofNodeUnpacked {
  Goal(GoalIdx),
  Candidate(CandidateIdx),
  Result(ResultIdx),
}

impl ProofNode {
  pub fn pack(pnu: ProofNodeUnpacked) -> ProofNode {
    let idx = match &pnu {
      ProofNodeUnpacked::Goal(goal_idx) => goal_idx.raw(),
      ProofNodeUnpacked::Candidate(candidate_idx) => candidate_idx.raw(),
      ProofNodeUnpacked::Result(result_idx) => result_idx.raw(),
    };
    assert!(idx & ((1 << 31) | (1 << 30)) == 0);
    ProofNode(match pnu {
      ProofNodeUnpacked::Goal(_) => idx,
      ProofNodeUnpacked::Candidate(_) => idx | (1 << 31),
      ProofNodeUnpacked::Result(_) => idx | (1 << 30),
    })
  }
  pub fn unpack(self) -> ProofNodeUnpacked {
    let idx = self.0 & (u32::MAX >> 2);
    if self.0 & ((1 << 31) | (1 << 30)) == 0 {
      ProofNodeUnpacked::Goal(GoalIdx::from_raw(idx))
    } else if self.0 & (1 << 31) != 0 {
      ProofNodeUnpacked::Candidate(CandidateIdx::from_raw(idx))
    } else {
      ProofNodeUnpacked::Result(ResultIdx::from_raw(idx))
    }
  }
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
  // TODO?
  ParamEnv(()),
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
  pub root: ProofNode,

  #[cfg_attr(feature = "testing", ts(type = "GoalData[]"))]
  pub goals: IndexVec<GoalIdx, GoalData>,

  #[cfg_attr(feature = "testing", ts(type = "CandidateData[]"))]
  pub candidates: IndexVec<CandidateIdx, CandidateData>,

  #[cfg_attr(feature = "testing", ts(type = "ResultData[]"))]
  pub results: IndexVec<ResultIdx, ResultData>,

  #[cfg_attr(feature = "testing", ts(type = "TyVal[]"))]
  pub tys: IndexVec<TyIdx, json::Value>,

  #[cfg_attr(feature = "testing", ts(type = "Implementors[]"))]
  pub implementors: IndexVec<ImplementorsIdx, Implementors>,

  pub impls: HashMap<GoalIdx, ImplementorsIdx>,

  pub projection_values: HashMap<TyIdx, TyIdx>,

  pub topology: GraphTopology,

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
  #[serde(rename = "trait")]
  pub trait_: json::Value,
  pub impls: Vec<CandidateIdx>,
  pub inductive_impls: Vec<CandidateIdx>,
}

#[derive(Serialize, Debug, Clone)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ProofCycle(Vec<ProofNode>);

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

#[cfg(all(test, feature = "testing"))]
#[test]
fn export_bindings_indices() {
  GoalIdx::export().expect("could not export type");
  ImplementorsIdx::export().expect("could not export type");
  CandidateIdx::export().expect("could not export type");
  ResultIdx::export().expect("could not export type");
}
