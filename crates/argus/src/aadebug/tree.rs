use std::time::Instant;

use index_vec::IndexVec;
use rustc_infer::infer::InferCtxt;
use rustc_middle::{
  traits::solve::{CandidateSource, Goal as RGoal},
  ty::{self, TyCtxt},
};
use rustc_trait_selection::solve::inspect::ProbeKind;
use rustc_utils::timer::elapsed;
use serde::Serialize;
#[cfg(feature = "testing")]
use ts_rs::TS;

use super::{ty as myty, util};
use crate::{
  analysis::EvaluationResult,
  ext::{EvaluationResultExt, TyCtxtExt},
  proof_tree::{topology::TreeTopology, ProofNodeIdx},
};

pub type I = ProofNodeIdx;

pub struct And(Vec<I>);
pub struct Dnf(Vec<And>);

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct SetHeuristic {
  pub total: usize,
  pub max_depth: usize,
  goals: Vec<Heuristic>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct Heuristic {
  idx: I,
  kind: GoalKind,
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
enum Location {
  Local,
  External,
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
enum GoalKind {
  Trait { _self: Location, _trait: Location },
  TyChange,
  FnToTrait { _trait: Location },
  TyAsCallable { arity: usize },
  FnParamDel { d: usize },
  Misc,
}

impl GoalKind {
  fn weight(&self) -> usize {
    use GoalKind::*;
    use Location::*;
    match self {
      Trait {
        _self: Local,
        _trait: Local,
      } => 0,

      Trait {
        _self: Local,
        _trait: External,
      } => 1,

      // NOTE: if the failed predicate is fn(..): Trait then treat the
      // function as an `External` type, because you can't implement traits
      // for functions, that has to be done via blanket impls using Fn traits.
      Trait {
        _self: External,
        _trait: Local,
      }
      // You can't implement a trait for function, they have to be
      // done by the crate exporting the trait.
      | FnToTrait { _trait: Local } => 2,

      Trait {
        _self: External,
        _trait: External,
      } => 3,
      FnToTrait { _trait: External } => 4,

      TyChange => 4,
      FnParamDel { d } => 4 * d,
      // You could implement the unstable Fn traits for a type,
      // we could thens suggest this if there's nothing else better.
      TyAsCallable { arity } => 10 + arity,
      Misc => 20,
    }
  }
}

impl And {
  fn distribute(&self, rhs: &Dnf) -> Dnf {
    Dnf(
      rhs
        .0
        .iter()
        .map(|And(rhs)| {
          And(self.0.iter().copied().chain(rhs.iter().copied()).collect())
        })
        .collect(),
    )
  }

  /// Failed predicates are weighted as follows.
  ///
  /// Each predicate is marked as local / external, local predicates are
  /// trusted less, while external predicates are assumed correct.
  ///
  /// Trait predicates `T: C`, are weighted by how much they could change.
  /// A type `T` that is local is non-rigid while external types are considered
  /// rigid, meaning they cannot be changed.
  ///
  /// Non-intrusive changes:
  ///
  /// A local type failing to implement a trait (local/external).
  /// NOTE that `T: C` where `T` is an external type is considered impossible
  /// to change, if this is the only option a relaxed rule might suggest
  /// creating a wapper for the type.
  ///
  /// Intrusive changes
  ///
  /// Changing types. That could either be changing a type to match an
  /// alias-relate, deleting function parameters or tuple elements.
  pub fn weight<'tcx>(self, tree: &T<'_, 'tcx>) -> SetHeuristic {
    let goals = self
      .0
      .iter()
      .map(|&idx| tree.goal(idx).expect("goal").analyze())
      .collect::<Vec<_>>();
    let total = goals.iter().fold(0, |acc, g| acc + g.kind.weight());
    SetHeuristic {
      total,
      goals,
      max_depth: self
        .0
        .iter()
        .map(|&idx| tree.topology.depth(idx))
        .max()
        .unwrap_or(0),
    }
  }
}

impl Dnf {
  pub fn into_iter_conjuncts(self) -> impl Iterator<Item = And> {
    self.0.into_iter()
  }

  fn or(vs: impl Iterator<Item = Self>) -> Option<Self> {
    let vs = vs.flat_map(|Self(v)| v).collect::<Vec<_>>();
    if vs.is_empty() {
      None
    } else {
      Some(Self(vs))
    }
  }

  fn distribute(self, other: Self) -> Self {
    Self::or(
      self
        .0
        .into_iter()
        .map(|conjunct| conjunct.distribute(&other)),
    )
    .expect("non-empty")
  }

  fn and(vs: impl Iterator<Item = Self>) -> Option<Self> {
    vs.reduce(Self::distribute)
  }

  fn single(i: I) -> Self {
    Self(vec![And(vec![i])])
  }
}

pub struct Goal<'a, 'tcx> {
  idx: I,
  result: EvaluationResult,
  tree: &'a T<'a, 'tcx>,
  infcx: &'a InferCtxt<'tcx>,
  goal: &'a RGoal<'tcx, ty::Predicate<'tcx>>,
}

impl Into<ProofNodeIdx> for Goal<'_, '_> {
  fn into(self) -> ProofNodeIdx {
    self.idx
  }
}

impl Into<ProofNodeIdx> for &Goal<'_, '_> {
  fn into(self) -> ProofNodeIdx {
    self.idx
  }
}

impl<'a, 'tcx> Goal<'a, 'tcx> {
  fn all_candidates(&self) -> impl Iterator<Item = Candidate<'a, 'tcx>> + '_ {
    self
      .tree
      .topology
      .children(self.idx)
      .filter_map(move |i| self.tree.candidate(i))
  }

  fn interesting_candidates(
    &self,
  ) -> impl Iterator<Item = Candidate<'a, 'tcx>> + '_ {
    self.all_candidates().filter(|c| c.retain)
  }

  pub fn predicate(&self) -> ty::Predicate<'tcx> {
    self.goal.predicate.clone()
  }

  pub fn last_ancestor_pre_builtin(&self) -> Self {
    let not_builtin = |kind| {
      !matches!(kind, ProbeKind::TraitCandidate {
        source: CandidateSource::BuiltinImpl(..),
        ..
      })
    };

    let mut i = self.idx;
    let tree = self.tree;

    while let Some(parent) = tree.topology.parent(i)
      && let N::C { kind, .. } = tree.ns[parent]
      && not_builtin(kind)
      && let Some(grandparent) = tree.topology.parent(parent)
    {
      i = grandparent;
    }

    tree.goal(i).expect("invalid ancestor")
  }

  fn as_trait_predicate(&self) -> Option<ty::PolyTraitPredicate<'tcx>> {
    let predicate = self.goal.predicate.kind();
    match predicate.skip_binder() {
      ty::PredicateKind::Clause(ty::ClauseKind::Trait(t)) => {
        Some(predicate.rebind(t))
      }
      _ => None,
    }
  }

  fn expect_trait_predicate(&self) -> ty::PolyTraitPredicate<'tcx> {
    self.as_trait_predicate().expect("trait-predicate")
  }

  fn analyze(&self) -> Heuristic {
    // We should only be analyzing failed predicates
    assert!(!self.result.is_yes());

    log::debug!("ANALYZING {:?}", self.predicate());

    let tcx = self.infcx.tcx;

    let kind = match self.predicate().kind().skip_binder() {
      ty::PredicateKind::Clause(ty::ClauseKind::Trait(t))
        if t.polarity == ty::PredicatePolarity::Positive
          && tcx.is_fn_trait(t.def_id())
          && let Some(fn_arity) = myty::function_arity(tcx, t.self_ty()) =>
      {
        let trait_arity = myty::fn_trait_arity(tcx, t).unwrap_or(usize::MAX);

        log::debug!("FnSigs\n{:?}\n{:?}", t.self_ty(), t.trait_ref);
        log::debug!("Fn Args {:?}", t.trait_ref.args.into_type_list(tcx));
        log::debug!("{} v {}", fn_arity, trait_arity);

        if fn_arity > trait_arity {
          GoalKind::FnParamDel {
            d: fn_arity - trait_arity,
          }
        } else {
          GoalKind::Misc
        }
      }

      // Self type is not callable but triat is in Fn family.
      ty::PredicateKind::Clause(ty::ClauseKind::Trait(t))
        if t.polarity == ty::PredicatePolarity::Positive
          && tcx.is_fn_trait(t.def_id()) =>
      {
        let trait_arity = myty::fn_trait_arity(tcx, t).unwrap_or(usize::MAX);
        GoalKind::TyAsCallable { arity: trait_arity }
      }

      // Self type is a function type but the trait isn't
      ty::PredicateKind::Clause(ty::ClauseKind::Trait(t))
        if t.polarity == ty::PredicatePolarity::Positive
          && let Some(_) = myty::function_arity(tcx, t.self_ty()) =>
      {
        let def_id = t.def_id();
        let location = if def_id.is_local() {
          Location::Local
        } else {
          Location::External
        };
        GoalKind::FnToTrait { _trait: location }
      }

      ty::PredicateKind::Clause(ty::ClauseKind::Trait(t))
        if t.polarity == ty::PredicatePolarity::Positive =>
      {
        log::debug!("Trait Self Ty {:?}", t.self_ty());

        let ty = t.self_ty();
        let def_id = t.def_id();

        let def_id_local = def_id.is_local();
        let ty_local = myty::is_local(ty);

        match (ty_local, def_id_local) {
          (true, true) => GoalKind::Trait {
            _self: Location::Local,
            _trait: Location::Local,
          },
          (true, false) => GoalKind::Trait {
            _self: Location::Local,
            _trait: Location::External,
          },
          (false, true) => GoalKind::Trait {
            _self: Location::External,
            _trait: Location::Local,
          },
          (false, false) => GoalKind::Trait {
            _self: Location::External,
            _trait: Location::External,
          },
        }
      }

      ty::PredicateKind::Clause(ty::ClauseKind::Trait(t)) => {
        log::debug!("Trait Self Ty {:?}", t.self_ty());
        GoalKind::Misc
      }

      ty::PredicateKind::Clause(ty::ClauseKind::Projection(_)) => {
        GoalKind::TyChange
      }

      ty::PredicateKind::Clause(..) => GoalKind::Misc,

      ty::PredicateKind::NormalizesTo(..)
      | ty::PredicateKind::AliasRelate(..)
      | ty::PredicateKind::ObjectSafe(..)
      | ty::PredicateKind::Subtype(..)
      | ty::PredicateKind::Coerce(..)
      | ty::PredicateKind::ConstEquate(..)
      | ty::PredicateKind::Ambiguous => GoalKind::Misc,
    };

    Heuristic {
      idx: self.idx,
      kind,
    }
  }
}

#[allow(dead_code)]
pub struct Candidate<'a, 'tcx> {
  idx: I,
  retain: bool,
  result: EvaluationResult,
  tree: &'a T<'a, 'tcx>,
  kind: &'a ProbeKind<TyCtxt<'tcx>>,
}

impl<'a, 'tcx> Candidate<'a, 'tcx> {
  fn all_subgoals(&self) -> impl Iterator<Item = Goal<'a, 'tcx>> + '_ {
    self
      .tree
      .topology
      .children(self.idx)
      .filter_map(move |i| self.tree.goal(i))
  }

  fn source_subgoals(&self) -> impl Iterator<Item = Goal<'a, 'tcx>> + '_ {
    use smallvec::SmallVec;
    let mut all_goals = self.all_subgoals().collect::<SmallVec<[_; 18]>>();

    let idx = itertools::partition(&mut all_goals, |g| {
      !g.result.is_yes() && g.as_trait_predicate().is_some()
    });

    let (trait_preds, _) = all_goals.split_at(idx);

    let is_implied_by_failing_bound = |other: &Goal<'_, 'tcx>| {
      trait_preds.iter().any(|bound| {
        if let ty::TraitPredicate {
          trait_ref,
          polarity: ty::PredicatePolarity::Positive,
        } = bound.expect_trait_predicate().skip_binder()
          // Don't consider reflexive implication
          && other.idx != bound.idx
        {
          other
            .infcx
            .tcx
            .does_trait_ref_occur_in(trait_ref, other.goal.predicate)
        } else {
          false
        }
      })
    };

    let mut to_keep = vec![];
    for (i, here) in all_goals.iter().enumerate() {
      if !is_implied_by_failing_bound(here) {
        log::debug!("Keeping Goal {:?}", here.goal.predicate);
        to_keep.push(i);
      }
    }

    util::pick_selected(all_goals, to_keep)
  }
}

pub enum N<'tcx> {
  C {
    kind: ProbeKind<TyCtxt<'tcx>>,
    result: EvaluationResult,
    retain: bool,
  },
  R {
    infcx: InferCtxt<'tcx>,
    goal: RGoal<'tcx, ty::Predicate<'tcx>>,
    result: EvaluationResult,
  },
}

pub struct T<'a, 'tcx: 'a> {
  pub root: I,
  pub ns: &'a IndexVec<I, N<'tcx>>,
  pub topology: &'a TreeTopology,
  pub maybe_ambiguous: bool,
}

impl<'a, 'tcx: 'a> T<'a, 'tcx> {
  pub fn goal(&self, i: I) -> Option<Goal<'_, 'tcx>> {
    match &self.ns[i] {
      N::R {
        infcx,
        goal,
        result,
      } => Some(Goal {
        idx: i,
        result: *result,
        tree: self,
        infcx,
        goal,
      }),
      _ => None,
    }
  }

  pub fn candidate(&self, i: I) -> Option<Candidate<'_, 'tcx>> {
    match &self.ns[i] {
      N::C {
        kind,
        result,
        retain,
      } => Some(Candidate {
        idx: i,
        retain: *retain,
        result: *result,
        tree: self,
        kind,
      }),
      _ => None,
    }
  }

  pub fn dnf(&self) -> Dnf {
    fn _goal<'tcx>(this: &T, goal: Goal<'_, 'tcx>) -> Option<Dnf> {
      if !match this.maybe_ambiguous {
        true => goal.result.is_maybe(),
        false => goal.result.is_no(),
      } {
        return None;
      }

      let candidates = goal.interesting_candidates();
      let nested = candidates
        .filter_map(|c| _candidate(this, c))
        .collect::<Vec<_>>();

      if nested.is_empty() {
        return Dnf::single(goal.idx).into();
      }

      Dnf::or(nested.into_iter())
    }

    fn _candidate<'tcx>(
      this: &T,
      candidate: Candidate<'_, 'tcx>,
    ) -> Option<Dnf> {
      if candidate.result.is_yes() {
        return None;
      }

      let goals = candidate.source_subgoals();
      let nested = goals.filter_map(|g| _goal(this, g)).collect::<Vec<_>>();

      if nested.is_empty() {
        return None;
      }

      Dnf::and(nested.into_iter())
    }

    let root = self.goal(self.root).expect("invalid root");
    _goal(self, root).unwrap_or_else(|| Dnf(vec![]))
  }

  pub fn iter_correction_sets(&self) -> impl Iterator<Item = And> {
    let tree_start = Instant::now();
    let iter = self.dnf().into_iter_conjuncts();
    elapsed("tree::dnf", tree_start);
    iter
  }
}

// ------------------
// Unimportant things

impl std::fmt::Debug for N<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      N::C {
        kind,
        result,
        retain,
      } => write!(f, "C {{ {} {:?} {:?} }}", retain, result, kind),
      N::R { goal, result, .. } => write!(
        f,
        "R {{ result: {:?}, goal: {:?} }}",
        result, goal.predicate
      ),
    }
  }
}
