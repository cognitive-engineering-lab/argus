use anyhow::{bail, Result};
use argus_ext::{
  rustc::InferCtxtExt,
  ty::{EvaluationResultExt, PredicateExt, TyExt},
};
use index_vec::IndexVec;
use rustc_hir::def_id::DefId;
use rustc_infer::infer::InferCtxt;
use rustc_middle::ty;
use rustc_span::Span;
use rustc_trait_selection::{
  solve::inspect::{
    InspectCandidate, InspectGoal, ProofTreeInferCtxtExt, ProofTreeVisitor,
  },
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
  pub all_impl_candidates: HashMap<ProofNodeIdx, Vec<CandidateIdx>>,

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
      all_impl_candidates: HashMap::default(),

      deferred_leafs: Vec::default(),
      interners: Interners::default(),
      aadebug: aadebug::Storage::new(maybe_ambiguous),
    }
  }

  fn check_goal_projection(&mut self, goal: &InspectGoal) {
    if goal.result().is_yes()
      && let ty::PredicateKind::AliasRelate(
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
      all_impl_candidates,
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
      all_impl_candidates,
      topology,
      cycle,
      analysis,
    })
  }

  // TODO: cycle detection is too expensive for large trees, and strictly
  // comparing the JSON values is a bad idea in general. (This is what comparing
  // interned keys does essentially). We should wait until the new trait solver
  // has some mechanism for detecting cycles and piggy back off that.
  // FIXME: this is currently disabled but we should check for cycles again...
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

impl<'tcx> SerializedTreeVisitor<'tcx> {
  /// Visited the nested subgoals of `candidate`, removing subgoals whose
  /// success depends on another failing goal..
  // TODO: See the commented out `InspectCandidateExt`, this should replace the below
  // function when the necessary changes have been upstreamed to rustc. That trait
  // will allow us to *only* traverse the subgoals to keep, rather than removed
  // goals after visiting them initially.
  fn visit_nested_roots(
    &mut self,
    tcx: ty::TyCtxt<'tcx>,
    candidate_idx: ProofNodeIdx,
    candidate: &InspectCandidate<'_, 'tcx>,
  ) -> <Self as ProofTreeVisitor<'tcx>>::Result {
    use crate::aadebug::tree as dgb_tree;

    // HACK: visit all nested candidates then remove them after the fact if they
    // shouldn't have been visited in the first place.
    candidate.visit_nested_in_probe(self);

    // After visiting nested subgoals, remove those from the tree that
    // depend on a failing subgoal.
    let subgoals = self
      .topology
      .children(candidate_idx)
      .collect::<smallvec::SmallVec<[_; 12]>>();

    // XXX: rust can't infer the more generic type for this, so it needs to get
    // annotated ... argh
    let get_result: &dyn for<'a> Fn(&'a ProofNodeIdx) -> EvaluationResult =
      &|&idx| match self.aadebug.ns[idx] {
        dgb_tree::N::R { result, .. } => result,
        dgb_tree::N::C { .. } => unreachable!(),
      };

    let error_sources = argus_ext::ty::identify_error_sources(
      &subgoals,
      get_result,
      |&idx| match self.aadebug.ns[idx] {
        dgb_tree::N::R { goal, .. } => goal.predicate,
        dgb_tree::N::C { .. } => unreachable!(),
      },
      move |_| tcx,
    )
    .collect::<smallvec::SmallVec<[_; 8]>>();

    for (i, subgoal) in subgoals.into_iter().enumerate() {
      if get_result(&subgoal).is_no() && !error_sources.contains(&i) {
        if let Some(v) = self.topology.children.get_mut(&candidate_idx) {
          v.retain(|&n| n != subgoal);
        }

        self.topology.parent.remove(&subgoal);
      }
    }
  }

  fn record_all_impls(
    &mut self,
    idx: ProofNodeIdx,
    goal: &InspectGoal<'_, 'tcx>,
  ) {
    // If the Goal is a TraitPredicate we will cache *all* possible implementors
    if let Some(tp) = goal.goal().predicate.as_trait_predicate() {
      let infcx = goal.infcx();
      for can in infcx.find_similar_impl_candidates(tp) {
        let can_idx = self.interners.intern_impl(infcx, can.impl_def_id);
        self
          .all_impl_candidates
          .entry(idx)
          .or_default()
          .push(can_idx);
      }
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

    // Record all the possible candidate impls for this goal.
    self.record_all_impls(here_idx, goal);

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

      self.visit_nested_roots(goal.infcx().tcx, candidate_idx, &c);

      // FIXME: is this necessary now that we store all nodes?
      add_result_if_empty(self, candidate_idx);
    }

    add_result_if_empty(self, here_idx);
    self.previous = here_parent;
  }
}

pub fn try_serialize<'tcx>(
  goal: solve::Goal<'tcx, ty::Predicate<'tcx>>,
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

// TODO: after we make the `visit_with` method public this can be a generic trait.
// trait InspectCandidateExt<'tcx> {
//   fn visit_nested_roots<V: ProofTreeVisitor<'tcx>>(
//     &self,
//     visitor: &mut V,
//   ) -> V::Result;
// }

// impl<'tcx> InspectCandidateExt<'tcx> for InspectCandidate<'_, 'tcx> {
//   fn visit_nested_roots<V: ProofTreeVisitor<'tcx>>(
//     &self,
//     visitor: &mut V,
//   ) -> V::Result {
//     // HACK: visit all nested candidates then remove them after the fact if they
//     // shouldn't have been visited in the first place.
//     self.visit_nested_in_probe(visitor);

//     // TODO: if we can lobby lcnr to make `visit_with` public then we don't have to visit
//     // all subgoals, only those that cause errors. This means that if `F: Fn()` fails, we
//     // don't need to check the bound `<F as FnOnce>::Output: ResBound`.
//     //
//     // If this gets used we no longer have to check this in the `aadebug` module.
//     //
//     // // TODO: add rustc_ast_ir to extern crates.
//     // use rustc_ast_ir::visit::VisitorResult;
//     // use rustc_ast_ir::try_visit;
//     //
//     // self.goal().infcx().probe(|_| {
//     //   let mut all_sub_goals = self.instantiate_nested_goals(visitor.span());
//     //   // Put all successful subgoals at the front of the list.
//     //   let err_start_idx = itertools::partition(&mut all_sub_goals, |g| g.result().is_yes());
//     //   let (successful_subgoals, failed_subgoals) = all_sub_goals.split_at_mut(err_start_idx);
//     //   // TODO: make a version of `retain_error_sources` that iterates over
//     //   // a slice and picks out the errors by index, then we can avoid the clone.
//     //   let mut failed_subgoals_vec = failed_subgoals.to_vec();
//     //   argus_ext::ty::retain_error_sources(
//     //     &mut failed_subgoals_vec,
//     //     |g| g.result(),
//     //     |g| g.goal().predicate,
//     //     |g| g.infcx().tcx,
//     //     |a, b| a.goal().predicate == b.goal().predicate,
//     //   );
//     //
//     //   for goal in failed_subgoals_vec.iter().chain(successful_subgoals.iter()) {
//     //     try_visit!(goal.visit_with(visitor));
//     //   }
//     //
//     //   V::Result::output()
//     // })
//   }
// }
