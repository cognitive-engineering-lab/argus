use anyhow::{bail, Result};
use argus_ext::{
  rustc::InferCtxtExt,
  ty::{EvaluationResultExt, ImplCandidateExt, PredicateExt, TyExt},
};
use index_vec::IndexVec;
use rustc_ast_ir::{try_visit, visit::VisitorResult};
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

pub struct SerializedTreeVisitor<'tcx> {
  pub root: Option<ProofNodeIdx>,
  pub previous: Option<ProofNodeIdx>,
  pub nodes: IndexVec<ProofNodeIdx, Node>,
  pub topology: TreeTopology,
  pub cycle: Option<ProofCycle>,
  pub projection_values: HashMap<TyIdx, TyIdx>,
  pub all_impl_candidates: HashMap<ProofNodeIdx, Implementors>,

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
    // We only care about successful alias relations
    if !goal.result().is_yes() {
      return;
    }

    let ty::PredicateKind::AliasRelate(
      t1,
      t2,
      ty::AliasRelationDirection::Equate,
    ) = goal.goal().predicate.kind().skip_binder()
    else {
      return;
    };

    if let (Some(mut t1), Some(mut t2)) = (t1.as_type(), t2.as_type()) {
      // Disallow projections involving two aliases
      if !(t1.is_alias() && t2.is_alias()) && t1 != t2 {
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
        }) {
          if t1 != t2 && !self.projection_values.contains_key(&t1) {
            let not_empty = self.projection_values.insert(t1, t2);
            debug_assert!(not_empty.is_none());
          }
        }
      }
    }
  }

  pub fn into_tree(self) -> Result<SerializedTree> {
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
  fn record_all_impls(
    &mut self,
    idx: ProofNodeIdx,
    goal: &InspectGoal<'_, 'tcx>,
  ) {
    // If the Goal is a TraitPredicate we will cache *all* possible implementors
    if let Some(tp) = goal.goal().predicate.as_trait_predicate() {
      let infcx = goal.infcx();
      let tcx = infcx.tcx;

      let identity_trait_ref =
        ty::TraitRef::identity(tcx, tp.skip_binder().trait_ref.def_id);

      let trait_ = ser::TraitRefPrintOnlyTraitPathDef(identity_trait_ref);
      let trait_ = tls::unsafe_access_interner(|ty_interner| {
        ser::to_value_expect(infcx, ty_interner, &trait_)
      });

      // Gather all impls
      let mut impls = vec![];
      let mut inductive_impls = vec![];
      let mut impl_candidates = infcx.find_similar_impl_candidates(tp);

      // HACK: Sort the `impl_candidates` by the number of *type* parameters. We use this
      // as a proxy for complexity, that is, complexity of reading the impl, we want
      // to show Argus users "simpler" impls first.
      // This probably shouldn't happen here, as it's a concern of the frontend, but this is
      // the last place we have all that information.
      macro_rules! sort_by_count {
        ($field:ident, $vec:expr) => {
          $vec.sort_by(|c1, c2| {
            let c1 = tcx.generics_of(c1.impl_def_id).own_counts().$field;
            let c2 = tcx.generics_of(c2.impl_def_id).own_counts().$field;
            c1.cmp(&c2)
          })
        };
      }
      sort_by_count!(types, impl_candidates);
      sort_by_count!(lifetimes, impl_candidates);

      for can in impl_candidates {
        let can_idx = self.interners.intern_impl(infcx, can.impl_def_id);
        if can.is_inductive(tcx) {
          inductive_impls.push(can_idx);
        } else {
          impls.push(can_idx);
        }
      }

      if !inductive_impls.is_empty() {
        log::trace!("inductive impls: {:?}", inductive_impls);
      }

      self.all_impl_candidates.insert(idx, Implementors {
        trait_,
        impls,
        inductive_impls,
      });
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

      c.visit_nested_roots(self);

      // FIXME: is this necessary now that we store all nodes?
      add_result_if_empty(self, candidate_idx);
    }

    add_result_if_empty(self, here_idx);
    self.previous = here_parent;
  }
}

// TODO: after we make the `visit_with` method public this can be a generic trait.
trait InspectCandidateExt<'tcx> {
  fn visit_nested_roots<V: ProofTreeVisitor<'tcx>>(
    &self,
    visitor: &mut V,
  ) -> V::Result;
}

impl<'tcx> InspectCandidateExt<'tcx> for InspectCandidate<'_, 'tcx> {
  fn visit_nested_roots<V: ProofTreeVisitor<'tcx>>(
    &self,
    visitor: &mut V,
  ) -> V::Result {
    self.goal().infcx().probe(|_| {
      let mut all_sub_goals = self.instantiate_nested_goals(visitor.span());
      // Put all successful subgoals at the front of the list.
      let err_start_idx =
        itertools::partition(&mut all_sub_goals, |g| g.result().is_yes());
      let (successful_subgoals, failed_subgoals) =
        all_sub_goals.split_at_mut(err_start_idx);

      let cap = argus_ext::ty::retain_error_sources(
        failed_subgoals,
        InspectGoal::result,
        |g| g.goal().predicate,
        |g| g.infcx().tcx,
      );

      for goal in failed_subgoals[.. cap]
        .iter()
        .chain(successful_subgoals.iter())
      {
        try_visit!(visitor.visit_goal(goal));
      }

      V::Result::output()
    })
  }
}
