use anyhow::{bail, Result};
use argus_ext::ty::{EvaluationResultExt, TyExt};
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
use crate::aadebug;

pub struct SerializedTreeVisitor<'tcx> {
  pub root: Option<ProofNodeIdx>,
  pub previous: Option<ProofNodeIdx>,
  pub nodes: IndexVec<ProofNodeIdx, Node>,
  pub topology: TreeTopology,
  pub cycle: Option<ProofCycle>,
  pub projection_values: HashMap<TyIdx, TyIdx>,

  deferred_leafs: Vec<(ProofNodeIdx, EvaluationResult)>,
  interners: Interners,
  aadebug: aadebug::Storage<'tcx>,
}

impl SerializedTreeVisitor<'_> {
  pub fn new(maybe_ambiguous: bool) -> Self {
    SerializedTreeVisitor {
      root: None,
      previous: None,
      nodes: IndexVec::default(),
      topology: TreeTopology::new(),
      cycle: None,
      projection_values: HashMap::default(),

      deferred_leafs: Vec::default(),
      interners: Interners::default(),
      aadebug: aadebug::Storage::new(maybe_ambiguous),
    }
  }

  fn check_goal_projection(&mut self, goal: &InspectGoal) {
    if let ty::PredicateKind::AliasRelate(
      t1,
      t2,
      ty::AliasRelationDirection::Equate,
    ) = goal.goal().predicate.kind().skip_binder()
      && let Some(mut t1) = t1.ty()
      && let Some(mut t2) = t2.ty()
      // Disallow projections involving two aliases
      && !(t1.is_alias() && t2.is_alias())
      && t1 != t2
    {
      if t2.is_alias() {
        // We want the map to go from alias -> concrete, swap the
        // types so that the alias is on the LHS. This doesn't change
        // the semantics because we only save `Equate` relations.
        std::mem::swap(&mut t1, &mut t2);
      }

      if let Some((t1, t2)) = crate::tls::unsafe_access_interner(|interner| {
        let idx1: TyIdx = interner.borrow().get_idx(&t1)?;
        let idx2: TyIdx = interner.borrow().get_idx(&t2)?;
        Some((idx1, idx2))
      }) && t1 != t2
      {
        let not_empty = self.projection_values.insert(t1, t2);
        debug_assert!(not_empty.is_none());
      }
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
      mut nodes,
      mut topology,
      cycle,
      projection_values,
      mut interners,
      aadebug,
      deferred_leafs,
      ..
    } = self
    else {
      bail!("missing root node!");
    };

    let analysis = aadebug.into_results(root, &topology);

    // Handle the deferred leafs (an inconvenience we'll deal with later)
    for (parent, res) in deferred_leafs {
      let leaf = interners.mk_result_node(res);
      let leaf_idx = nodes.push(leaf);
      topology.add(parent, leaf_idx);
    }

    let (goals, candidates, results) = interners.take();
    let tys = crate::tls::take_interned_tys();

    Ok(SerializedTree {
      root,
      nodes,
      goals,
      candidates,
      results,
      tys,
      projection_values,
      topology,
      cycle,
      analysis,
    })
  }

  // TODO: cycle detection is too expensive for large trees, and strictly
  // comparing the JSON values is a bad idea in general. (This is what comparing
  // interned keys does essentially). We should wait until the new trait solver
  // has some mechanism for detecting cycles and piggy back off that.
  // FIXME: this is currently dissabled but we should check for cycles again...
  #[allow(dead_code)]
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

impl<'tcx> ProofTreeVisitor<'tcx> for SerializedTreeVisitor<'tcx> {
  type Result = ();

  fn span(&self) -> Span {
    rustc_span::DUMMY_SP
  }

  fn visit_goal(&mut self, goal: &InspectGoal<'_, 'tcx>) -> Self::Result {
    log::trace!("visit_goal {:?}", goal.goal());

    let here_node = self.interners.mk_goal_node(goal);
    let here_idx = self.nodes.push(here_node);
    // Push node into the analysis tree.
    self.aadebug.push_goal(here_idx, goal).unwrap();

    // After interning the goal we can check whether or not
    // it's an successful alias relate predicate for two types.
    self.check_goal_projection(goal);

    if self.root.is_none() {
      self.root = Some(here_idx);
    }

    if let Some(prev) = self.previous {
      self.topology.add(prev, here_idx);
    }

    // Check if there was an "overflow" from the freshly added node,
    // XXX: this is largely a HACK for right now; it ignores
    // how the solver actually works, and is ignorant of inference vars.
    // self.check_for_cycle_from(here_idx);

    let here_parent = self.previous;

    let add_result_if_empty = |this: &mut Self, n: ProofNodeIdx| {
      if this.topology.is_leaf(n) {
        this.deferred_leafs.push((n, goal.result()));
      }
    };

    for c in goal.candidates() {
      let here_candidate = self.interners.mk_candidate_node(&c);
      let candidate_idx = self.nodes.push(here_candidate);
      self
        .aadebug
        .push_candidate(candidate_idx, goal, &c)
        .unwrap();

      self.topology.add(here_idx, candidate_idx);
      self.previous = Some(candidate_idx);
      c.visit_nested_in_probe(self);
      // FIXME: is this necessary now that we store all nodes?
      add_result_if_empty(self, candidate_idx);
    }

    add_result_if_empty(self, here_idx);
    self.previous = here_parent;
  }
}

pub fn try_serialize<'tcx>(
  goal: solve::Goal<'tcx, Predicate<'tcx>>,
  result: EvaluationResult,
  span: Span,
  infcx: &InferCtxt<'tcx>,
  _def_id: DefId,
) -> Result<SerializedTree> {
  super::format::dump_proof_tree(goal, span, infcx);

  infcx.probe(|_| {
    let mut visitor = SerializedTreeVisitor::new(result.is_maybe());
    infcx.visit_proof_tree(goal, &mut visitor);
    visitor.into_tree()
  })
}
