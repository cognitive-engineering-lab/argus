use std::time::Instant;

use argus_ext::ty::{EvaluationResultExt, TyCtxtExt, TyExt};
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

use crate::{
  analysis::EvaluationResult,
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
  pub momentum: usize,
  pub velocity: usize,
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
    use GoalKind as GK;
    use Location::{External as E, Local as L};
    match self {
      GK::Trait {
        _self: L,
        _trait: L,
      } => 0,

      GK::Trait {
        _self: L,
        _trait: E,
      } => 1,

      // NOTE: if the failed predicate is fn(..): Trait then treat the
      // function as an `External` type, because you can't implement traits
      // for functions, that has to be done via blanket impls using Fn traits.
      GK::Trait {
        _self: E,
        _trait: L,
      }
      // You can't implement a trait for function, they have to be
      // done by the crate exporting the trait.
      | GK::FnToTrait { _trait: L } => 2,

      GK::Trait {
        _self: E,
        _trait: E,
      } => 3,

      GK::FnToTrait { _trait: E }
      | GK::TyChange => 4,
      GK::FnParamDel { d } => 4 * d,

      // You could implement the unstable Fn traits for a type,
      // we could thens suggest this if there's nothing else better.
      GK::TyAsCallable { arity } => 10 + arity,
      GK::Misc => 20,
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
  pub fn weight(self, tree: &T) -> SetHeuristic {
    let goals = self
      .0
      .iter()
      .map(|&idx| tree.goal(idx).expect("goal").analyze())
      .collect::<Vec<_>>();

    let momentum = goals.iter().fold(0, |acc, g| acc + g.kind.weight());
    let velocity = self
      .0
      .iter()
      .map(|&idx| tree.topology.depth(idx))
      .max()
      .unwrap_or(0);

    SetHeuristic {
      momentum,
      velocity,
      goals,
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

  #[allow(clippy::needless_pass_by_value)]
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

#[allow(clippy::struct_field_names)]
pub struct Goal<'a, 'tcx> {
  idx: I,
  result: EvaluationResult,
  tree: &'a T<'a, 'tcx>,
  infcx: &'a InferCtxt<'tcx>,
  goal: &'a RGoal<'tcx, ty::Predicate<'tcx>>,
}

impl From<Goal<'_, '_>> for I {
  fn from(val: Goal) -> Self {
    val.idx
  }
}

impl From<&Goal<'_, '_>> for I {
  fn from(val: &Goal) -> Self {
    val.idx
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
    self.goal.predicate
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

  fn analyze(&self) -> Heuristic {
    // We should only be analyzing failed predicates
    assert!(!self.result.is_yes());

    log::debug!("ANALYZING {:?}", self.predicate());

    let tcx = self.infcx.tcx;

    let kind = match self.predicate().kind().skip_binder() {
      ty::PredicateKind::Clause(ty::ClauseKind::Trait(t))
        if t.polarity == ty::PredicatePolarity::Positive
          && tcx.is_fn_trait(t.def_id())
          && let Some(fn_arity) = tcx.function_arity(t.self_ty()) =>
      {
        let trait_arity = tcx.fn_trait_arity(t).unwrap_or(usize::MAX);

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
        let trait_arity = tcx.fn_trait_arity(t).unwrap_or(usize::MAX);
        GoalKind::TyAsCallable { arity: trait_arity }
      }

      // Self type is a function type but the trait isn't
      ty::PredicateKind::Clause(ty::ClauseKind::Trait(t))
        if t.polarity == ty::PredicatePolarity::Positive
          && let Some(_) = tcx.function_arity(t.self_ty()) =>
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
        let ty_local = ty.is_local();

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

      ty::PredicateKind::Clause(..)
      | ty::PredicateKind::NormalizesTo(..)
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
    let mut all_goals = self.all_subgoals().collect::<Vec<_>>();
    argus_ext::ty::retain_error_sources(
      &mut all_goals,
      |g| g.result,
      |g| g.goal.predicate,
      |g| g.infcx.tcx,
      |a, b| a.idx == b.idx,
    );

    all_goals.into_iter()
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
      N::C { .. } => None,
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
      N::R { .. } => None,
    }
  }

  pub fn dnf(&self) -> Dnf {
    fn _goal(this: &T, goal: &Goal) -> Option<Dnf> {
      if !((this.maybe_ambiguous && goal.result.is_maybe())
        || goal.result.is_no())
      {
        return None;
      }

      let candidates = goal.interesting_candidates();
      let nested = candidates
        .filter_map(|c| _candidate(this, &c))
        .collect::<Vec<_>>();

      if nested.is_empty() {
        return Dnf::single(goal.idx).into();
      }

      Dnf::or(nested.into_iter())
    }

    fn _candidate(this: &T, candidate: &Candidate) -> Option<Dnf> {
      if candidate.result.is_yes() {
        return None;
      }

      let goals = candidate.source_subgoals();
      let nested = goals.filter_map(|g| _goal(this, &g)).collect::<Vec<_>>();

      if nested.is_empty() {
        return None;
      }

      Dnf::and(nested.into_iter())
    }

    let root = self.goal(self.root).expect("invalid root");
    _goal(self, &root).unwrap_or_else(|| Dnf(vec![]))
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
      } => write!(f, "C {{ {retain} {result:?} {kind:?} }}"),
      N::R { goal, result, .. } => {
        write!(f, "R {{ result: {result:?}, goal: {:?} }}", goal.predicate)
      }
    }
  }
}
