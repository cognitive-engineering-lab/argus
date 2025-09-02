use anyhow::{Result, bail};
use argus_ext::ty::{EvaluationResultExt, PredicateExt, TyExt};
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

use super::{
  interners::{InternedData, Interners},
  *,
};
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
  pub root: Option<ProofNode>,
  pub previous: Option<ProofNode>,
  pub topology: GraphTopology,
  pub cycle: Option<ProofCycle>,
  pub projection_values: HashMap<TyIdx, TyIdx>,

  impls: HashMap<GoalIdx, ImplementorsIdx>,
  interners: Interners,
  aadebug: aadebug::Storage<'tcx>,
}

impl SerializedTreeVisitor<'_> {
  pub fn new(maybe_ambiguous: bool) -> Self {
    SerializedTreeVisitor {
      root: None,
      previous: None,
      topology: GraphTopology::new(),
      cycle: None,
      projection_values: HashMap::default(),

      impls: HashMap::default(),
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
        }) && t1 != t2
          && !self.projection_values.contains_key(&t1)
        {
          let not_empty = self.projection_values.insert(t1, t2);
          debug_assert!(not_empty.is_none());
        }
      }
    }
  }

  pub fn into_tree(self) -> Result<SerializedTree> {
    let SerializedTreeVisitor {
      root: Some(root),
      topology,
      cycle,
      projection_values,
      impls,
      interners,
      aadebug,
      ..
    } = self
    else {
      bail!("missing root node!");
    };

    let analysis = aadebug.into_results(root, &topology);

    let InternedData {
      goals,
      implementors,
      candidates,
      results,
    } = interners.take();
    let tys = crate::tls::take_interned_tys();

    Ok(SerializedTree {
      root,
      goals,
      candidates,
      results,
      tys,
      implementors,
      impls,
      projection_values,
      topology,
      cycle,
      analysis,
    })
  }
}

impl<'tcx> SerializedTreeVisitor<'tcx> {
  fn record_all_impls(
    &mut self,
    goal_idx: GoalIdx,
    goal: &InspectGoal<'_, 'tcx>,
  ) {
    // If the Goal is a TraitPredicate we will cache *all* possible implementors
    if let Some(tp) = goal.goal().predicate.as_trait_predicate() {
      let def_id = tp.def_id();
      let infcx = goal.infcx();
      let impls_idx = self.interners.intern_implementors(infcx, def_id, tp);
      self.impls.insert(goal_idx, impls_idx);
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

    // Record all the possible candidate impls for this goal.
    if let ProofNodeUnpacked::Goal(goal_idx) = here_node.unpack() {
      self.record_all_impls(goal_idx, goal);
    }

    // Push node into the analysis tree.
    self.aadebug.push_goal(here_node, goal);

    // After interning the goal we can check whether or not
    // it's an successful alias relate predicate for two types.
    self.check_goal_projection(goal);

    if self.root.is_none() {
      self.root = Some(here_node);
    }

    if let Some(prev) = self.previous {
      self.topology.add(prev, here_node);
    }

    let here_parent = self.previous;

    for c in goal.candidates() {
      let here_candidate = self.interners.mk_candidate_node(&c);
      if self.topology.children.contains_key(&here_candidate) {
        continue;
      }
      self.aadebug.push_candidate(here_candidate, goal, &c);

      self.topology.add(here_node, here_candidate);
      self.previous = Some(here_candidate);

      c.visit_nested_roots(self);
    }

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
