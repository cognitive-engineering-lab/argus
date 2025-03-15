//! Functionality coming from Rustc.
//!
//! These are things that we might be able to convince people to make
//! public within Rustc itself, but our needs may change so it hasn't
//! happened yet.
//!
//! The goal is that each copied block of code is modified minimally,
//! making replacement easier in the future.
use rustc_infer::{
  infer::{self, InferCtxt},
  traits::{
    query::NoSolution, MismatchedProjectionTypes, PredicateObligation,
    SelectionError,
  },
};
use rustc_middle::ty::{
  self,
  error::{ExpectedFound, TypeError},
  ToPolyTraitRef,
};
use rustc_span::DUMMY_SP;
use rustc_trait_selection::{
  infer::InferCtxtExt as RustcInferCtxtExt,
  traits::{elaborate, FulfillmentError, FulfillmentErrorCode},
};

use crate::EvaluationResult;

pub mod fn_ctx;

#[derive(Debug)]
#[allow(dead_code)]
pub struct ImplCandidate<'tcx> {
  pub trait_ref: ty::TraitRef<'tcx>,
  pub similarity: CandidateSimilarity,
  pub impl_def_id: rustc_span::def_id::DefId,
}

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub enum CandidateSimilarity {
  Exact { ignoring_lifetimes: bool },
  Fuzzy { ignoring_lifetimes: bool },
  Other,
}

macro_rules! bug {
  ($( $tree:tt ),*) => {
    panic!( $( $tree )* )
  }
}

pub trait InferCtxtExt<'tcx> {
  fn can_match_projection(
    &self,
    goal: ty::ProjectionPredicate<'tcx>,
    assumption: ty::PolyProjectionPredicate<'tcx>,
  ) -> bool;

  fn to_fulfillment_error(
    &self,
    obligation: &PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> Option<FulfillmentError<'tcx>>;

  fn can_match_trait(
    &self,
    goal: ty::TraitPredicate<'tcx>,
    assumption: ty::PolyTraitPredicate<'tcx>,
  ) -> bool;

  fn error_implies(
    &self,
    cond: ty::Predicate<'tcx>,
    error: ty::Predicate<'tcx>,
  ) -> bool;

  fn find_similar_impl_candidates(
    &self,
    trait_pred: ty::PolyTraitPredicate<'tcx>,
  ) -> Vec<ImplCandidate<'tcx>>;

  fn fuzzy_match_tys(
    &self,
    a: ty::Ty<'tcx>,
    b: ty::Ty<'tcx>,
    ignoring_lifetimes: bool,
  ) -> Option<CandidateSimilarity>;
}

impl<'tcx> InferCtxtExt<'tcx> for InferCtxt<'tcx> {
  fn can_match_trait(
    &self,
    goal: ty::TraitPredicate<'tcx>,
    assumption: ty::PolyTraitPredicate<'tcx>,
  ) -> bool {
    if goal.polarity != assumption.polarity() {
      return false;
    }

    let trait_goal = goal.trait_ref;
    let trait_assumption = self.instantiate_binder_with_fresh_vars(
      DUMMY_SP,
      infer::BoundRegionConversionTime::HigherRankedType,
      assumption.to_poly_trait_ref(),
    );

    self.can_eq(ty::ParamEnv::empty(), trait_goal, trait_assumption)
  }

  // FIXME: there is no longer a single `to_error` function making this logic outdated.
  #[allow(clippy::match_same_arms)]
  fn to_fulfillment_error(
    &self,
    obligation: &PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> Option<FulfillmentError<'tcx>> {
    let infcx = self;
    let obligation = obligation.clone();

    match result {
      Err(NoSolution) => {}
      _ => return None,
    }

    Some(
      // Taken from rustc_trait_selection::solve::fulfill.rs
      FulfillmentError {
        obligation: obligation.clone(),
        code: match obligation.predicate.kind().skip_binder() {
          ty::PredicateKind::Clause(ty::ClauseKind::Projection(_)) => {
            FulfillmentErrorCode::Project(
              // FIXME: This could be a `Sorts` if the term is a type
              MismatchedProjectionTypes {
                err: TypeError::Mismatch,
              },
            )
          }
          ty::PredicateKind::NormalizesTo(..) => {
            FulfillmentErrorCode::Project(MismatchedProjectionTypes {
              err: TypeError::Mismatch,
            })
          }
          ty::PredicateKind::AliasRelate(_, _, _) => {
            FulfillmentErrorCode::Project(MismatchedProjectionTypes {
              err: TypeError::Mismatch,
            })
          }
          ty::PredicateKind::Subtype(pred) => {
            let (a, b) = infcx.enter_forall_and_leak_universe(
              obligation.predicate.kind().rebind((pred.a, pred.b)),
            );
            let expected_found = ExpectedFound::new(a, b);
            FulfillmentErrorCode::Subtype(
              expected_found,
              TypeError::Sorts(expected_found),
            )
          }
          ty::PredicateKind::Coerce(pred) => {
            let (a, b) = infcx.enter_forall_and_leak_universe(
              obligation.predicate.kind().rebind((pred.a, pred.b)),
            );
            let expected_found = ExpectedFound::new(b, a);
            FulfillmentErrorCode::Subtype(
              expected_found,
              TypeError::Sorts(expected_found),
            )
          }
          ty::PredicateKind::Clause(_)
          | ty::PredicateKind::DynCompatible(_)
          | ty::PredicateKind::Ambiguous => {
            FulfillmentErrorCode::Select(SelectionError::Unimplemented)
          }
          ty::PredicateKind::ConstEquate(..) => {
            bug!("unexpected goal: {obligation:?}")
          }
        },
        root_obligation: obligation,
      },
    )
  }

  fn can_match_projection(
    &self,
    goal: ty::ProjectionPredicate<'tcx>,
    assumption: ty::PolyProjectionPredicate<'tcx>,
  ) -> bool {
    let assumption = self.instantiate_binder_with_fresh_vars(
      DUMMY_SP,
      infer::BoundRegionConversionTime::HigherRankedType,
      assumption,
    );

    let param_env = ty::ParamEnv::empty();
    self.can_eq(param_env, goal.projection_term, assumption.projection_term)
      && self.can_eq(param_env, goal.term, assumption.term)
  }

  fn error_implies(
    &self,
    cond: ty::Predicate<'tcx>,
    error: ty::Predicate<'tcx>,
  ) -> bool {
    if cond == error {
      return true;
    }

    if let Some(error) = error.as_trait_clause() {
      self.enter_forall(error, |error| {
        elaborate(self.tcx, std::iter::once(cond))
          .filter_map(|implied| implied.as_trait_clause())
          .any(|implied| self.can_match_trait(error, implied))
      })
    } else if let Some(error) = error.as_projection_clause() {
      self.enter_forall(error, |error| {
        elaborate(self.tcx, std::iter::once(cond))
          .filter_map(|implied| implied.as_projection_clause())
          .any(|implied| self.can_match_projection(error, implied))
      })
    } else {
      false
    }
  }

  fn find_similar_impl_candidates(
    &self,
    trait_pred: ty::PolyTraitPredicate<'tcx>,
  ) -> Vec<ImplCandidate<'tcx>> {
    let mut candidates: Vec<_> = self
      .tcx
      .all_impls(trait_pred.def_id())
      .filter_map(|def_id| {
        let imp = self.tcx.impl_trait_header(def_id).unwrap();
        if imp.polarity == ty::ImplPolarity::Negative
          || !self.tcx.is_user_visible_dep(def_id.krate)
        {
          return None;
        }
        let imp = imp.trait_ref.skip_binder();

        self
          .fuzzy_match_tys(
            trait_pred.skip_binder().self_ty(),
            imp.self_ty(),
            false,
          )
          .map(|similarity| ImplCandidate {
            trait_ref: imp,
            similarity,
            impl_def_id: def_id,
          })
          .or(Some(ImplCandidate {
            trait_ref: imp,
            similarity: CandidateSimilarity::Other,
            impl_def_id: def_id,
          }))
      })
      .collect();
    if candidates
      .iter()
      .any(|c| matches!(c.similarity, CandidateSimilarity::Exact { .. }))
    {
      // If any of the candidates is a perfect match, we don't want to show all of them.
      // This is particularly relevant for the case of numeric types (as they all have the
      // same category).
      candidates
        .retain(|c| matches!(c.similarity, CandidateSimilarity::Exact { .. }));
    }
    candidates
  }

  fn fuzzy_match_tys(
    &self,
    mut a: ty::Ty<'tcx>,
    mut b: ty::Ty<'tcx>,
    ignoring_lifetimes: bool,
  ) -> Option<CandidateSimilarity> {
    /// returns the fuzzy category of a given type, or None
    /// if the type can be equated to any type.
    fn type_category(tcx: ty::TyCtxt<'_>, t: ty::Ty<'_>) -> Option<u32> {
      match t.kind() {
        ty::Bool => Some(0),
        ty::Char => Some(1),
        ty::Str => Some(2),
        ty::Adt(def, _) if Some(def.did()) == tcx.lang_items().string() => {
          Some(2)
        }
        ty::Int(..)
        | ty::Uint(..)
        | ty::Float(..)
        | ty::Infer(ty::IntVar(..) | ty::FloatVar(..)) => Some(4),
        ty::Ref(..) | ty::RawPtr(..) => Some(5),
        ty::Array(..) | ty::Slice(..) => Some(6),
        ty::FnDef(..) | ty::FnPtr(..) => Some(7),
        ty::Dynamic(..) => Some(8),
        ty::Closure(..) => Some(9),
        ty::Tuple(..) => Some(10),
        ty::Param(..) => Some(11),
        ty::Alias(ty::Projection, ..) => Some(12),
        ty::Alias(ty::Inherent, ..) => Some(13),
        ty::Alias(ty::Opaque, ..) => Some(14),
        ty::Alias(ty::Weak, ..) => Some(15),
        ty::Never => Some(16),
        ty::Adt(..) => Some(17),
        ty::Coroutine(..) => Some(18),
        ty::Foreign(..) => Some(19),
        ty::CoroutineWitness(..) => Some(20),
        ty::CoroutineClosure(..) => Some(21),
        ty::Pat(..) => Some(22),
        ty::Placeholder(..) | ty::Bound(..) | ty::Infer(..) | ty::Error(_) => {
          None
        }
      }
    }

    let strip_references = |mut t: ty::Ty<'tcx>| -> ty::Ty<'tcx> {
      loop {
        match t.kind() {
          ty::Ref(_, inner, _) | ty::RawPtr(inner, _) => t = *inner,
          _ => break t,
        }
      }
    };

    if !ignoring_lifetimes {
      a = strip_references(a);
      b = strip_references(b);
    }

    let cat_a = type_category(self.tcx, a)?;
    let cat_b = type_category(self.tcx, b)?;
    if a == b {
      Some(CandidateSimilarity::Exact { ignoring_lifetimes })
    } else if cat_a == cat_b {
      match (a.kind(), b.kind()) {
        (ty::Adt(def_a, _), ty::Adt(def_b, _)) => def_a == def_b,
        (ty::Foreign(def_a), ty::Foreign(def_b)) => def_a == def_b,
        // Matching on references results in a lot of unhelpful
        // suggestions, so let's just not do that for now.
        //
        // We still upgrade successful matches to `ignoring_lifetimes: true`
        // to prioritize that impl.
        (ty::Ref(..) | ty::RawPtr(..), ty::Ref(..) | ty::RawPtr(..)) => {
          self.fuzzy_match_tys(a, b, true).is_some()
        }
        _ => true,
      }
      .then_some(CandidateSimilarity::Fuzzy { ignoring_lifetimes })
    } else if ignoring_lifetimes {
      None
    } else {
      self.fuzzy_match_tys(a, b, true)
    }
  }
}
