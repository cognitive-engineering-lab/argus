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

use crate::serialize::{
  hir::Option__ImplDef, serialize_to_value, ty::goal__predicate_def,
};

crate::define_idx! {
  usize,
  ProofNodeIdx
}

#[derive(Serialize, TS, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Node {
  Result { data: String },
  Candidate { data: Candidate },
  Goal { data: Goal },
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
  #[ts(type = "any")]
  goal: serde_json::Value,
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Candidate {
  Impl {
    #[ts(type = "Impl | undefined")]
    data: serde_json::Value,
    fallback: String,
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
}

// ----------------------------------------
// impls

impl Goal {
  fn new<'tcx>(
    infcx: &InferCtxt<'tcx>,
    goal: &solve::Goal<'tcx, ty::Predicate<'tcx>>,
  ) -> Self {
    #[derive(Serialize)]
    struct Wrapper<'a, 'tcx: 'a>(
      #[serde(serialize_with = "goal__predicate_def")]
      &'a solve::Goal<'tcx, ty::Predicate<'tcx>>,
    );

    let goal = serialize_to_value(infcx, &Wrapper(goal))
      .expect("failed to serialize goal");
    Self { goal }
  }
}

impl Candidate {
  fn new_impl<'tcx, 'hir>(
    infcx: &InferCtxt<'tcx>,
    impl_: Option<&'hir hir::Impl<'hir>>,
    fallback: String,
  ) -> Self {
    #[derive(Serialize)]
    struct Wrapper<'hir>(
      #[serde(skip_serializing_if = "Option::is_none")]
      #[serde(with = "Option__ImplDef")]
      Option<&'hir hir::Impl<'hir>>,
    );

    let impl_ = serialize_to_value(infcx, &Wrapper(impl_))
      .expect("couldn't serialie impl");

    Self::Impl {
      data: impl_,
      fallback,
    }
  }
}

impl From<&'static str> for Candidate {
  fn from(value: &'static str) -> Self {
    Candidate::Any {
      data: value.to_string(),
    }
  }
}
