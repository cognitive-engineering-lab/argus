mod tree;
mod ty;
mod util;

use std::time::Instant;

use anyhow::Result;
use index_vec::IndexVec;
use rustc_data_structures::fx::FxHashMap as HashMap;
use rustc_infer::traits::solve::GoalSource;
use rustc_trait_selection::solve::inspect::{InspectCandidate, InspectGoal};
use rustc_utils::timer::elapsed;
use serde::Serialize;
#[cfg(feature = "testing")]
use ts_rs::TS;

use crate::{
  ext::EvaluationResultExt,
  proof_tree::{topology::TreeTopology, ProofNodeIdx},
};

pub struct Storage<'tcx> {
  pub ns: IndexVec<ProofNodeIdx, tree::N<'tcx>>,
  maybe_ambiguous: bool,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct AnalysisResults {
  problematic_sets: Vec<tree::SetHeuristic>,
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
    let tree = &tree::T {
      root,
      ns: &self.ns,
      topology: topo,
      maybe_ambiguous: false,
    };

    let tree_start = Instant::now();

    let mut sets = HashMap::<usize, Vec<_>>::default();

    for conjunct in tree.iter_correction_sets() {
      let h = conjunct.weight(tree);
      sets.entry(h.total).or_default().push(h);
    }

    for (_, group) in sets.iter_mut() {
      group.sort_by_key(|g| -(g.max_depth as i32));
    }

    let mut sets = sets.into_values().flatten().collect::<Vec<_>>();
    sets.sort_by_key(|s| s.total);

    elapsed("aadeg::into_results", tree_start);

    AnalysisResults {
      problematic_sets: sets,
    }
  }
}

// ==========================
// OLD CODE FOR FUDGING TYPES
// ==========================

// let failed_predicates = conjunct.iter(tree).collect::<Vec<_>>();

// let predicate_tys = failed_predicates
//   .iter()
//   .map(|g| g.predicate())
//   .collect::<HashSet<_>>();

// // For all failed trait predicates:
// // 1. build a type that implements the trait
// // 2. find the last ancestor before a "built-in" impl (or the root)
// // 3. group predicates by the chosen ancestor.
// // 4. for each ancestor, substitute all the chosen types and evaluate the substitution.
// //    NOTE: we don't substitute types for inference variables. We want to see if these
// //    get naturally resolved by the other substitutions. If not, we include their weights.
// let type_substs = failed_predicates
//   .iter()
//   .map(|g| LazyCell::new(move || tree.find_type_substitution_for(g)))
//   .collect::<Vec<_>>();

// let by_last_ancestor = failed_predicates
//   .iter()
//   .enumerate()
//   .map(|(i, g)| {
//     let ancestor: ProofNodeIdx = g.last_ancestor_pre_builtin().into();
//     (ancestor, (i, g))
//   })
//   .into_group_map();

// // The weight is an estimate of how much work it would take
// // to fix all the failed predicates in the group.
// //
// // Weight 1: changing a type to implement a trait.
// //   TODO: this can be further refined by "how much" a type needs to be
// //   adjusted in order to implement a trait. Deferring this for now.
// // Weight 2: changing an *inference variable* to implement a trait.
// // Weight 3: changing a type to *be* another type.
// //   NOTE change for normalization, not trait implementation
// let (subs_worked, weight, conjunct_steps) =
//   by_last_ancestor.into_iter().fold(
//     (true, 0, vec![]),
//     |(mut subs_worked, mut weight, mut conjunct_steps), (root, group)| {
//       // All type substitutions will be applied in this context.
//       // let tree::N::R {
//       //   goal,
//       //   infcx,
//       //   result: original_result,
//       // } = &self.ns[root]
//       // else {
//       //   unreachable!();
//       // };

//       // let type_substitutions = group
//       //   .iter()
//       //   .filter_map(|(i, _)| *type_substs[*i])
//       //   .collect::<Vec<_>>();

//       // let group_len = group.len();
//       // let sub_len = type_substitutions.len();
//       // let result =
//       //   infcx.eval_substitution(&type_substitutions, goal.clone());

//       // let (my_success, my_weight) = if result.is_yes() {
//       //   // If a substitution worked then all predicates have weight 1.
//       //   (true, sub_len)
//       // } else if result.is_better_than(original_result) {
//       //   // If it's "better," but still not a "yes," then
//       //   // we weight each predicate by two.
//       //   (true, group_len * 2 - sub_len)
//       // } else {
//       //   (false, group_len * 3)
//       // };

//       // subs_worked &= my_success;
//       // weight += my_weight;

//       // FIXME: remove hardcoded values
//       conjunct_steps.push(GroupChange {
//         nodes: group.iter().map(|(_, g)| (*g).into()).collect(),
//         weight: 0,                      // my_weight,
//         result: EvaluationResult::no(), //
//       });

//       // (subs_worked, weight, conjunct_steps)
//       (true, weight, conjunct_steps)
//     },
//   );

// problematic_sets.push(ConjunctAnalysis {
//   indices: conjunct_steps,
//   weight,
// });
