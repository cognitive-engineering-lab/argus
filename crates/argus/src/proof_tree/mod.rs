//! Proof tree types sent to the Argus frontend.

pub mod ext;
pub(super) mod serialize;
pub mod topology;

use std::collections::HashSet;

use index_vec::IndexVec;
use rustc_infer::infer::InferCtxt;
use rustc_middle::ty;
use rustc_trait_selection::traits::solve;
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
  ProofNodeIdx
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
  Result {
    #[serde(with = "EvaluationResultDef")]
    #[cfg_attr(feature = "testing", ts(type = "EvaluationResult"))]
    data: EvaluationResult,
  },
  Candidate {
    data: Candidate,
  },
  Goal {
    data: Goal,
  },
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct Goal {
  #[cfg_attr(feature = "testing", ts(type = "GoalPredicate"))]
  goal: serde_json::Value,

  #[serde(with = "EvaluationResultDef")]
  #[cfg_attr(feature = "testing", ts(type = "EvaluationResult"))]
  result: EvaluationResult,

  necessity: ObligationNecessity,
  num_vars: usize,
  is_lhs_ty_var: bool,

  #[cfg(debug_assertions)]
  #[cfg_attr(feature = "testing", ts(type = "string | undefined"))]
  debug_comparison: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum Candidate {
  Impl {
    #[cfg_attr(feature = "testing", ts(type = "ImplHeader"))]
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

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct SerializedTree {
  pub root: ProofNodeIdx,
  #[cfg_attr(feature = "testing", ts(type = "Node[]"))]
  pub nodes: IndexVec<ProofNodeIdx, Node>,
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

impl Goal {
  fn new<'tcx>(
    infcx: &InferCtxt<'tcx>,
    goal: &solve::Goal<'tcx, ty::Predicate<'tcx>>,
    result: EvaluationResult,
  ) -> Self {
    let necessity = infcx.guess_predicate_necessity(&goal.predicate);
    let goal = infcx.resolve_vars_if_possible(*goal);
    let num_vars =
      serialize::var_counter::count_vars(infcx.tcx, goal.predicate);

    let goal_value = serialize_to_value(infcx, &GoalPredicateDef(goal))
      .expect("failed to serialize goal");

    Self {
      goal: goal_value,
      result,
      necessity,
      num_vars,
      is_lhs_ty_var: goal.predicate.is_lhs_ty_var(),

      #[cfg(debug_assertions)]
      debug_comparison: format!("{:?}", goal.predicate.kind().skip_binder()),
    }
  }
}

impl Candidate {
  fn new_impl_header<'tcx>(
    infcx: &InferCtxt<'tcx>,
    impl_: &ImplHeader<'tcx>,
  ) -> Self {
    let impl_ =
      serialize_to_value(infcx, impl_).expect("couldn't serialize impl header");

    Self::Impl { data: impl_ }
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
