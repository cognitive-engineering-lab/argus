//! Proof tree types sent to the Argus frontend.

pub mod ext;
pub(super) mod serialize;
pub mod topology;

use std::collections::HashSet;

use index_vec::IndexVec;
use rustc_hir as hir;
use rustc_infer::infer::InferCtxt;
use rustc_middle::ty;
use rustc_trait_selection::traits::solve;
use serde::Serialize;
pub use topology::*;
use ts_rs::TS;

use crate::{
  ext::InferCtxtExt,
  serialize::{hir::ImplDef, serialize_to_value, ty::Goal__PredicateDef},
  types::{ImplHeader, ObligationNecessity},
};

crate::define_idx! {
  usize,
  ProofNodeIdx
}

#[derive(Serialize, TS, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Node {
  Result { data: String },
  Candidate { data: Candidate },
  Goal { data: Goal },
}

// FIXME: this shouldn't be EQ
#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
  #[ts(type = "any")]
  goal: serde_json::Value,
  necessity: ObligationNecessity,
}

// FIXME: this shouldn't be EQ
#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Candidate {
  ImplHir {
    #[ts(type = "Impl")]
    data: serde_json::Value,
  },
  ImplMiddle {
    #[ts(type = "any")]
    // Type is ImplHeader from mod `crate::types`.
    data: serde_json::Value,
  },
  ParamEnv {
    idx: usize,
  },
  // TODO remove variant once everything is structured
  Any {
    data: String,
  },
}

#[derive(Serialize, TS, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SerializedTree {
  pub root: ProofNodeIdx,
  pub nodes: IndexVec<ProofNodeIdx, Node>,
  pub topology: TreeTopology,
  pub error_leaves: Vec<ProofNodeIdx>,
  pub unnecessary_roots: HashSet<ProofNodeIdx>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cycle: Option<ProofCycle>,
}

#[derive(Serialize, TS, Debug, Clone)]
pub struct ProofCycle(Vec<ProofNodeIdx>);

// ----------------------------------------
// impls

impl Goal {
  fn new<'tcx>(
    infcx: &InferCtxt<'tcx>,
    goal: &solve::Goal<'tcx, ty::Predicate<'tcx>>,
  ) -> Self {
    #[derive(Serialize)]
    struct Wrapper<'a, 'tcx: 'a>(
      #[serde(with = "Goal__PredicateDef")]
      &'a solve::Goal<'tcx, ty::Predicate<'tcx>>,
    );

    let necessity = infcx.guess_predicate_necessity(&goal.predicate);
    let goal = serialize_to_value(infcx, &Wrapper(goal))
      .expect("failed to serialize goal");
    Self { goal, necessity }
  }
}

impl Candidate {
  fn new_impl_hir<'tcx, 'hir>(
    infcx: &InferCtxt<'tcx>,
    impl_: &'hir hir::Impl<'hir>,
  ) -> Self {
    #[derive(Serialize)]
    struct Wrapper<'hir>(#[serde(with = "ImplDef")] &'hir hir::Impl<'hir>);

    let impl_ = serialize_to_value(infcx, &Wrapper(impl_))
      .expect("couldn't serialize impl");

    Self::ImplHir { data: impl_ }
  }

  fn new_impl_header<'tcx>(
    infcx: &InferCtxt<'tcx>,
    impl_: &ImplHeader<'tcx>,
  ) -> Self {
    let impl_ =
      serialize_to_value(infcx, impl_).expect("couldn't serialize impl header");

    Self::ImplMiddle { data: impl_ }
  }

  // TODO: we should pass the ParamEnv here for certainty.
  fn new_param_env(idx: usize) -> Self {
    Self::ParamEnv { idx }
  }
}

impl From<&'static str> for Candidate {
  fn from(value: &'static str) -> Self {
    value.to_string().into()
  }
}

impl From<String> for Candidate {
  fn from(value: String) -> Self {
    Candidate::Any { data: value }
  }
}
