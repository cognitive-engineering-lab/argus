use std::{collections::HashSet, ops::ControlFlow};

use anyhow::{bail, Result};
use ext::{CandidateExt, EvaluationResultExt};
use index_vec::IndexVec;
use rustc_hir::def_id::DefId;
use rustc_infer::infer::InferCtxt;
use rustc_middle::ty::Predicate;
use rustc_trait_selection::{
  solve::inspect::{InspectGoal, ProofTreeInferCtxtExt, ProofTreeVisitor},
  traits::solve,
};

use super::{interners::Interners, *};

pub struct SerializedTreeVisitor {
  pub def_id: DefId,
  pub root: Option<ProofNodeIdx>,
  pub previous: Option<ProofNodeIdx>,
  pub nodes: IndexVec<ProofNodeIdx, Node>,
  pub topology: TreeTopology,
  pub error_leaves: Vec<ProofNodeIdx>,
  pub unnecessary_roots: HashSet<ProofNodeIdx>,
  pub cycle: Option<ProofCycle>,
  interners: Interners,
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
      cycle: None,
      interners: Interners::default(),
    }
  }

  pub fn into_tree(self) -> Result<SerializedTree> {
    let SerializedTreeVisitor {
      root: Some(root),
      nodes,
      topology,
      error_leaves,
      unnecessary_roots,
      cycle,
      interners,
      ..
    } = self
    else {
      bail!("missing root node!");
    };

    let (goals, candidates, results) = interners.take();

    Ok(SerializedTree {
      root,
      goals,
      candidates,
      results,
      nodes,
      topology,
      error_leaves,
      unnecessary_roots,
      cycle,
    })
  }

  // TODO: cycle detection is too expensive for large trees, and strictly
  // comparing the JSON values is a bad idea in general. (This is what comparing
  // interned keys does essentially). We should wait until the new trait solver
  // has some mechanism for detecting cycles and piggy back off that.
  fn check_for_cycle_from(&mut self, from: ProofNodeIdx) {
    if self.cycle.is_some() {
      return;
    }

    let Node::Goal(from_idx, result) = &self.nodes[from] else {
      return;
    };

    let to_root = self.topology.path_to_root(from);
    if to_root.iter_exclusive().any(|idx| {
      let Node::Goal(here_idx, hresult) = &self.nodes[*idx] else {
        return false;
      };

      here_idx == from_idx && hresult == result
    }) {
      self.cycle = Some(to_root.into());
    }
  }
}

impl<'tcx> ProofTreeVisitor<'tcx> for SerializedTreeVisitor {
  type BreakTy = !;

  fn visit_goal(
    &mut self,
    goal: &InspectGoal<'_, 'tcx>,
  ) -> ControlFlow<Self::BreakTy> {
    let here_node = self.interners.mk_goal_node(goal);

    let here_idx = self.nodes.push(here_node.clone());

    if self.root.is_none() {
      self.root = Some(here_idx);
    }

    if let Some(prev) = self.previous {
      self.topology.add(prev, here_idx);
    }

    // Check if there was an "overflow" from the freshly added node,
    // XXX: this is largely a HACK for right now; it ignores
    // how the solver actually works, and is ignorant of inference vars.
    self.check_for_cycle_from(here_idx);

    let prev = self.previous.clone();
    self.previous = Some(here_idx);

    for c in goal.candidates() {
      let here_candidate = self.interners.mk_candidate_node(&c);
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
        let leaf = self.interners.mk_result_node(result);
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

pub fn serialize_proof_tree<'tcx>(
  goal: solve::Goal<'tcx, Predicate<'tcx>>,
  infcx: &InferCtxt<'tcx>,
  def_id: DefId,
) -> Result<SerializedTree> {
  infcx.probe(|_| {
    let mut visitor = SerializedTreeVisitor::new(def_id);
    infcx.visit_proof_tree(goal, &mut visitor);
    visitor.into_tree()
  })
}

pub(super) mod var_counter {
  use rustc_middle::ty::{TyCtxt, TypeFoldable, TypeSuperFoldable};
  use rustc_type_ir::fold::TypeFolder;

  use super::*;

  pub fn count_vars<'tcx, T>(tcx: TyCtxt<'tcx>, t: T) -> usize
  where
    T: TypeFoldable<TyCtxt<'tcx>>,
  {
    let mut folder = TyVarCounterVisitor { tcx, count: 0 };
    t.fold_with(&mut folder);
    folder.count
  }

  pub(super) struct TyVarCounterVisitor<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub count: usize,
  }

  impl<'tcx> TypeFolder<TyCtxt<'tcx>> for TyVarCounterVisitor<'tcx> {
    fn interner(&self) -> TyCtxt<'tcx> {
      self.tcx
    }

    fn fold_ty(&mut self, ty: ty::Ty<'tcx>) -> ty::Ty<'tcx> {
      match ty.kind() {
        ty::Infer(ty::TyVar(_))
        | ty::Infer(ty::IntVar(_))
        | ty::Infer(ty::FloatVar(_)) => self.count += 1,
        _ => {}
      };
      ty.super_fold_with(self)
    }

    fn fold_const(&mut self, c: ty::Const<'tcx>) -> ty::Const<'tcx> {
      match c.kind() {
        ty::ConstKind::Infer(_) => self.count += 1,
        _ => {}
      };
      c.super_fold_with(self)
    }
  }
}
