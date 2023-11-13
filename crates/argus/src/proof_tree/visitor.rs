//! A replacement of the current public API for ProofTrees

use std::ops::ControlFlow;

use rustc_infer::infer::InferCtxt;
use rustc_middle::traits::query::NoSolution;
use rustc_middle::traits::solve::{inspect, QueryResult};
use rustc_middle::traits::solve::{Certainty, Goal};
use rustc_middle::ty;

use rustc_trait_selection::solve::inspect::ProofTreeBuilder;
use rustc_trait_selection::solve::{GenerateProofTree, InferCtxtEvalExt};

/// The public API to interact with proof trees.
pub trait ProofTreeVisitor<'tcx> {
    type BreakTy;

    fn visit_goal(&mut self, goal: &InspectGoal<'_, 'tcx>) -> ControlFlow<Self::BreakTy>;
}

// ---

pub trait ProofTreeInferCtxtExt<'tcx> {
    fn visit_proof_tree<V: ProofTreeVisitor<'tcx>>(
        &self,
        goal: Goal<'tcx, ty::Predicate<'tcx>>,
        visitor: &mut V,
    ) -> ControlFlow<V::BreakTy>;
}

impl<'tcx> ProofTreeInferCtxtExt<'tcx> for InferCtxt<'tcx> {
    fn visit_proof_tree<V: ProofTreeVisitor<'tcx>>(
        &self,
        goal: Goal<'tcx, ty::Predicate<'tcx>>,
        visitor: &mut V,
    ) -> ControlFlow<V::BreakTy> {
        let (_, proof_tree) = self.evaluate_root_goal(goal, GenerateProofTree::Yes);
        let proof_tree = proof_tree.unwrap();
        visitor.visit_goal(&InspectGoal::new(self, 0, &proof_tree))
    }
}
