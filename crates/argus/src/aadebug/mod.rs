mod dnf;
pub(crate) mod tree;

use std::time::Instant;

use anyhow::Result;
use argus_ext::ty::EvaluationResultExt;
use index_vec::IndexVec;
use rustc_infer::traits::solve::GoalSource;
use rustc_trait_selection::solve::inspect::{InspectCandidate, InspectGoal};
use rustc_utils::timer;
use serde::Serialize;
#[cfg(feature = "testing")]
use ts_rs::TS;

use crate::proof_tree::{topology::TreeTopology, ProofNodeIdx};

pub struct Storage<'tcx> {
  pub ns: IndexVec<ProofNodeIdx, tree::N<'tcx>>,
  maybe_ambiguous: bool,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct AnalysisResults {
  pub problematic_sets: Vec<tree::SetHeuristic>,
}

impl<'tcx> Storage<'tcx> {
  pub fn new(maybe_ambiguous: bool) -> Self {
    Self {
      ns: IndexVec::new(),
      maybe_ambiguous,
    }
  }

  pub fn push_goal(
    &mut self,
    idx: ProofNodeIdx,
    goal: &InspectGoal<'_, 'tcx>,
  ) -> Result<()> {
    let infcx = goal.infcx().fork();
    let result = goal.result();
    let goal = goal.goal();
    let new_idx = self.ns.push(tree::N::R {
      infcx,
      goal,
      result,
    });

    // TODO: the topology is stored elsewhere, we need to make
    // sure that the indices remain the same as the serialized tree.
    // In the future we can make this more type-safe by having a single
    // DS that holds both vectors. (Or more if we choose to employ multiple analyses.)
    if new_idx != idx {
      anyhow::bail!("Indices are out of sync {new_idx:?} != {idx:?}");
    }

    Ok(())
  }

  pub fn push_candidate(
    &mut self,
    idx: ProofNodeIdx,
    goal: &InspectGoal<'_, 'tcx>,
    candidate: &InspectCandidate<'_, 'tcx>,
  ) -> Result<()> {
    let retain = (self.maybe_ambiguous && candidate.result().is_ok())
      || goal.infcx().probe(|_| {
        candidate
          .instantiate_nested_goals(rustc_span::DUMMY_SP)
          .iter()
          .any(|nested_goal| {
            matches!(
              nested_goal.source(),
              GoalSource::ImplWhereBound | GoalSource::InstantiateHigherRanked
            ) && if self.maybe_ambiguous {
              nested_goal.result().is_maybe()
            } else {
              nested_goal.result().is_no()
            }
          })
      });

    let new_idx = self.ns.push(tree::N::C {
      kind: candidate.kind(),
      result: candidate.result(),
      retain,
    });

    // TODO: the topology is stored elsewhere, we need to make
    // sure that the indices remain the same as the serialized tree.
    // In the future we can make this more type-safe by having a single
    // DS that holds both vectors. (Or more if we choose to employ multiple analyses.)
    if new_idx != idx {
      anyhow::bail!("Indices are out of sync {new_idx:?} != {idx:?}");
    }

    Ok(())
  }

  pub fn into_results(
    self,
    root: ProofNodeIdx,
    topo: &TreeTopology,
  ) -> AnalysisResults {
    let tree = &tree::T::new(root, &self.ns, topo, false);
    let tree_start = Instant::now();

    let mut sets = vec![];
    tree.for_correction_set(|conjunct| {
      sets.push(tree.weight(&conjunct));
    });

    timer::elapsed("aadeg::into_results", tree_start);

    AnalysisResults {
      problematic_sets: sets,
    }
  }
}
