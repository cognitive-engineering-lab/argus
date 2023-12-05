use std::ops::ControlFlow;

use std::collections::HashSet;
// use rustc_hash::FxHashSet as HashSet;

use rustc_hir::{BodyId, FnSig};
use rustc_hir::def_id::DefId;
use rustc_hir_analysis::astconv::AstConv;
use rustc_hir_typeck::{FnCtxt, Inherited};
use rustc_infer::infer::InferCtxt;
use rustc_middle::ty::{TyCtxt, Predicate};

use rustc_trait_selection::solve::inspect::{ProofTreeVisitor, InspectGoal, InspectCandidate, ProofTreeInferCtxtExt};
use rustc_trait_selection::traits::{ObligationCtxt, FulfillmentError};
use rustc_trait_selection::traits::solve::{Goal, QueryInput, Certainty, MaybeCause};
use rustc_trait_selection::traits::query::NoSolution;
use rustc_type_ir::Canonical;

use rustc_utils::mir::body::BodyExt;

use serde::Serialize;
use index_vec::IndexVec;


use ext::*;
use super::*;

pub struct SerializedTreeVisitor {
    pub def_id: DefId,
    pub root: Option<ProofNodeIdx>,
    pub previous: Option<ProofNodeIdx>,
    pub nodes: IndexVec<ProofNodeIdx, Node>,
    pub topology: TreeTopology<ProofNodeIdx>,
    pub error_leaves: Vec<ProofNodeIdx>,
    pub unnecessary_roots: HashSet<ProofNodeIdx>,
}

impl SerializedTreeVisitor {
    pub fn new(def_id: DefId) -> Self {
        SerializedTreeVisitor {
            def_id,
            root: None,
            previous: None,
            nodes: IndexVec::default(),
            topology: TreeTopology::new(),
            error_leaves: Vec::default(),
            unnecessary_roots: HashSet::default(),
        }
    }

    pub fn into_tree(self) -> Option<SerializedTree> {
        let SerializedTreeVisitor {
            root: Some(root),
            nodes,
            topology,
            error_leaves,
            unnecessary_roots,
            ..
        } = self else {
            return None;
        };

        Some(SerializedTree {
            root,
            nodes,
            topology,
            error_leaves,
            unnecessary_roots,
        })
    }
}

impl SerializedTreeVisitor {}

impl Node {
    fn from_goal(goal: &InspectGoal, def_id: DefId) -> Self {
        let infcx = goal.infcx();
        let data = goal.goal().predicate.pretty(infcx, def_id);
        Node::Goal { data }
    }

    fn from_candidate(candidate: &InspectCandidate, def_id: DefId) -> Self {
        let infcx = candidate.infcx();
        let data = candidate.pretty(infcx, def_id);
        Node::Candidate { data }
    }

    fn from_result(result: &Result<Certainty, NoSolution>) -> Self {
        Node::Result { data: result.pretty() }
    }
}

impl ProofTreeVisitor<'_> for SerializedTreeVisitor {
    type BreakTy = !;

    fn visit_goal(
        &mut self,
        goal: &InspectGoal<'_, '_>
    ) -> ControlFlow<Self::BreakTy> {
        let infcx = goal.infcx();

        // TODO: we don't need to actually store/mark unnecessary roots atm.
        // The frontend doesn't use them, but eventually we will!
        // self.unnecessary_roots.insert(n);

        if !goal.goal().predicate.is_necessary(&infcx.tcx) {
            return ControlFlow::Continue(());
        }

        let here_node = Node::from_goal(goal, self.def_id);

        let here_idx = self.nodes.push(here_node.clone());

        if self.root.is_none() {
            self.root = Some(here_idx);
        }

        if let Some(prev) = self.previous {
            self.topology.add(prev, here_idx);
        }

        let prev = self.previous.clone();
        self.previous = Some(here_idx);

        for c in goal.candidates() {

            let here_candidate = Node::from_candidate(&c, self.def_id);
            let candidate_idx = self.nodes.push(here_candidate);

            let prev_idx = if c.is_informative_probe() {
                self.topology.add(here_idx, candidate_idx);
                self.previous = Some(candidate_idx);
                candidate_idx
            } else {
                here_idx
            };

            c.visit_nested(self)?;

            if self.topology.is_leaf(prev_idx) {
                let result = goal.result();
                let leaf = Node::from_result(&result);
                let leaf_idx = self.nodes.push(leaf);
                self.topology.add(prev_idx, leaf_idx);
                if !result.is_yes() {
                    self.error_leaves.push(leaf_idx);
                }
            }
        }

        self.previous = prev;

        ControlFlow::Continue(())
    }
}

pub fn serialize_proof_tree<'tcx>(goal: Goal<'tcx, Predicate<'tcx>>, infcx: &InferCtxt<'tcx>, def_id: DefId) -> Option<SerializedTree> {
    infcx.probe(|_| {
      let mut visitor = SerializedTreeVisitor::new(def_id);
      infcx.visit_proof_tree(goal, &mut visitor);
      visitor.into_tree()
  })
}
