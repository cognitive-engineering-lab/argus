use std::collections::HashSet;

use anyhow::{bail, Result};
use index_vec::IndexVec;
use rustc_hir::def_id::DefId;
use rustc_infer::infer::InferCtxt;
use rustc_middle::ty::Predicate;
use rustc_span::Span;
use rustc_trait_selection::{
  solve::inspect::{InspectGoal, ProofTreeInferCtxtExt, ProofTreeVisitor},
  traits::solve,
};

use super::{interners::Interners, *};

pub struct SerializedTreeVisitor {
  pub root: Option<ProofNodeIdx>,
  pub previous: Option<ProofNodeIdx>,
  pub nodes: IndexVec<ProofNodeIdx, Node>,
  pub topology: TreeTopology,
  pub unnecessary_roots: HashSet<ProofNodeIdx>,
  pub cycle: Option<ProofCycle>,
  interners: Interners,
}

impl SerializedTreeVisitor {
  pub fn new() -> Self {
    SerializedTreeVisitor {
      root: None,
      previous: None,
      nodes: IndexVec::default(),
      topology: TreeTopology::new(),
      unnecessary_roots: HashSet::default(),
      cycle: None,
      interners: Interners::default(),
    }
  }

  #[cfg(debug_assertions)]
  fn is_valid(&self) -> Result<()> {
    for (pidx, node) in self.nodes.iter_enumerated() {
      match node {
        Node::Goal(g) => {
          anyhow::ensure!(
            !self.topology.is_leaf(pidx),
            "non-leaf node (goal) has no children {:?}",
            self.interners.goal(*g)
          );
        }
        Node::Candidate(c) => {
          anyhow::ensure!(
            !self.topology.is_leaf(pidx),
            "non-leaf node (candidate) has no children {:?}",
            self.interners.candidate(*c)
          );
        }
        Node::Result(..) => {
          anyhow::ensure!(
            self.topology.is_leaf(pidx),
            "result node is not a leaf"
          );
        }
      }
    }
    Ok(())
  }

  pub fn into_tree(self) -> Result<SerializedTree> {
    #[cfg(debug_assertions)]
    self.is_valid()?;

    let SerializedTreeVisitor {
      root: Some(root),
      nodes,
      topology,
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

    let to_root = self.topology.path_to_root(from);
    let from_node = self.nodes[from];
    if to_root
      .iter_exclusive()
      .any(|middle| self.nodes[*middle] == from_node)
    {
      self.cycle = Some(to_root.into());
    }
  }
}

impl<'tcx> ProofTreeVisitor<'tcx> for SerializedTreeVisitor {
  type Result = ();

  fn span(&self) -> Span {
    rustc_span::DUMMY_SP
  }

  fn visit_goal(&mut self, goal: &InspectGoal<'_, 'tcx>) -> Self::Result {
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

    let here_parent = self.previous.clone();

    let add_result_if_empty = |this: &mut Self, n: ProofNodeIdx| {
      if this.topology.is_leaf(n) {
        let result = goal.result();
        let leaf = this.interners.mk_result_node(result);
        let leaf_idx = this.nodes.push(leaf);
        this.topology.add(n, leaf_idx);
      }
    };

    for c in goal.candidates() {
      let here_candidate = self.interners.mk_candidate_node(&c);
      let candidate_idx = self.nodes.push(here_candidate);
      self.topology.add(here_idx, candidate_idx);
      self.previous = Some(candidate_idx);
      c.visit_nested_in_probe(self);
      add_result_if_empty(self, candidate_idx);
    }

    add_result_if_empty(self, here_idx);
    self.previous = here_parent;
  }
}

pub fn serialize_proof_tree<'tcx>(
  goal: solve::Goal<'tcx, Predicate<'tcx>>,
  span: Span,
  infcx: &InferCtxt<'tcx>,
  _def_id: DefId,
) -> Result<SerializedTree> {
  super::format::dump_proof_tree(goal, span, infcx);

  infcx.probe(|_| {
    let mut visitor = SerializedTreeVisitor::new();
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
