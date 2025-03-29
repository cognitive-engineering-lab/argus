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

type RustcCandidateSimilarity =
  rustc_trait_selection::error_reporting::traits::CandidateSimilarity;

impl From<RustcCandidateSimilarity> for CandidateSimilarity {
  fn from(similarity: RustcCandidateSimilarity) -> Self {
    match similarity {
      RustcCandidateSimilarity::Exact { .. } => CandidateSimilarity::Exact {
        ignoring_lifetimes: false,
      },
      RustcCandidateSimilarity::Fuzzy { .. } => CandidateSimilarity::Fuzzy {
        ignoring_lifetimes: false,
      },
    }
  }
}

macro_rules! bug {
  ($( $tree:tt ),*) => {
    panic!( $( $tree )* )
  }
}

pub trait InferCtxtExt<'tcx> {
  /// Argus defined helper
  fn to_fulfillment_error(
    &self,
    obligation: &PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> Option<FulfillmentError<'tcx>>;

  /// Private in TypeErrCtxt
  fn can_match_projection(
    &self,
    goal: ty::ProjectionPredicate<'tcx>,
    assumption: ty::PolyProjectionPredicate<'tcx>,
  ) -> bool;

  /// Private in TypeErrCtxt
  fn can_match_trait(
    &self,
    goal: ty::TraitPredicate<'tcx>,
    assumption: ty::PolyTraitPredicate<'tcx>,
  ) -> bool;

  /// Private in TypeErrCtxt
  fn error_implies(
    &self,
    cond: ty::Predicate<'tcx>,
    error: ty::Predicate<'tcx>,
  ) -> bool;

  /// Private in TypeErrorCtxt
  fn find_similar_impl_candidates(
    &self,
    trait_pred: ty::PolyTraitPredicate<'tcx>,
  ) -> Vec<ImplCandidate<'tcx>>;

  /// Public (wrapping for local `CandidateSimilarity`)
  fn fuzzy_match_tys(
    &self,
    a: ty::Ty<'tcx>,
    b: ty::Ty<'tcx>,
    ignoring_lifetimes: bool,
  ) -> Option<CandidateSimilarity>;
}

impl<'tcx> InferCtxtExt<'tcx> for InferCtxt<'tcx> {
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

  fn can_match_trait(
    &self,
    goal: ty::TraitPredicate<'tcx>,
    assumption: ty::PolyTraitPredicate<'tcx>,
  ) -> bool {
    // Fast path
    if goal.polarity != assumption.polarity() {
      return false;
    }

    let trait_assumption = self.instantiate_binder_with_fresh_vars(
      DUMMY_SP,
      infer::BoundRegionConversionTime::HigherRankedType,
      assumption,
    );

    self.can_eq(
      ty::ParamEnv::empty(),
      goal.trait_ref,
      trait_assumption.trait_ref,
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

        if imp.polarity != ty::ImplPolarity::Positive
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
    a: ty::Ty<'tcx>,
    b: ty::Ty<'tcx>,
    ignoring_lifetimes: bool,
  ) -> Option<CandidateSimilarity> {
    use rustc_trait_selection::error_reporting::InferCtxtErrorExt;
    self
      .err_ctxt()
      .fuzzy_match_tys(a, b, ignoring_lifetimes)
      .map(CandidateSimilarity::from)
  }
}
