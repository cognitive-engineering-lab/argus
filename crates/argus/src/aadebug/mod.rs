mod dnf;
pub(crate) mod tree;

use std::time::Instant;

use argus_ext::ty::EvaluationResultExt;
use rustc_data_structures::fx::FxHashMap as HashMap;
use rustc_infer::traits::solve::GoalSource;
use rustc_trait_selection::solve::inspect::{InspectCandidate, InspectGoal};
use rustc_utils::timer;
use serde::Serialize;
#[cfg(feature = "testing")]
use ts_rs::TS;

use crate::proof_tree::{ProofNode, topology::GraphTopology};

pub struct Storage<'tcx> {
  pub ns: HashMap<ProofNode, tree::N<'tcx>>,
  maybe_ambiguous: bool,
  report_performance: bool,
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
    let report_performance = std::env::var("ARGUS_DNF_PERF").is_ok();
    Self {
      ns: HashMap::default(),
      maybe_ambiguous,
      report_performance,
    }
  }

  pub fn push_goal(
    &mut self,
    proof_node: ProofNode,
    goal: &InspectGoal<'_, 'tcx>,
  ) {
    let infcx = Box::new(goal.infcx().fork());
    let result = goal.result();
    let goal = goal.goal();
    self.ns.insert(proof_node, tree::N::R {
      infcx,
      goal,
      result,
    });
  }

  pub fn push_candidate(
    &mut self,
    proof_node: ProofNode,
    goal: &InspectGoal<'_, 'tcx>,
    candidate: &InspectCandidate<'_, 'tcx>,
  ) {
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

    self.ns.insert(proof_node, tree::N::C {
      kind: candidate.kind(),
      result: candidate.result(),
      retain,
    });
  }

  pub fn into_results(
    self,
    root: ProofNode,
    topo: &GraphTopology,
  ) -> AnalysisResults {
    let tree =
      &tree::T::new(root, &self.ns, topo, false, self.report_performance);
    let tree_start = Instant::now();

    let mut sets = vec![];
    tree.for_correction_set(|conjunct| sets.push(tree.weight(conjunct)));

    timer::elapsed("aadeg::into_results", tree_start);

    AnalysisResults {
      problematic_sets: sets,
    }
  }
}
