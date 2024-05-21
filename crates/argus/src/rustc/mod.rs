//! Functionality coming from Rustc.
//!
//! These are things that we might be able to convince people to make
//! public within Rustc itself, but our needs may change so it hasn't
//! happened yet.
//!
//! The goal is that each copied block of code is modified minimally,
//! making replacement easier in the future.

use rustc_infer::{
  infer::InferCtxt,
  traits::{
    query::NoSolution, FulfillmentError, FulfillmentErrorCode,
    MismatchedProjectionTypes, PredicateObligation, SelectionError,
  },
};
use rustc_middle::ty::{
  self,
  error::{ExpectedFound, TypeError},
  ToPolyTraitRef,
};
use rustc_span::DUMMY_SP;
use rustc_trait_selection::{infer, traits::elaborate};

use crate::types::intermediate::EvaluationResult;

pub mod fn_ctx;

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

  // TODO: there is no longer a single `to_error` route so this is outdated.
  fn to_fulfillment_error(
    &self,
    obligation: &PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> Option<FulfillmentError<'tcx>> {
    let infcx = self;
    let goal = obligation;
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
            let expected_found = ExpectedFound::new(true, a, b);
            FulfillmentErrorCode::Subtype(
              expected_found,
              TypeError::Sorts(expected_found),
            )
          }
          ty::PredicateKind::Coerce(pred) => {
            let (a, b) = infcx.enter_forall_and_leak_universe(
              obligation.predicate.kind().rebind((pred.a, pred.b)),
            );
            let expected_found = ExpectedFound::new(false, a, b);
            FulfillmentErrorCode::Subtype(
              expected_found,
              TypeError::Sorts(expected_found),
            )
          }
          ty::PredicateKind::Clause(_)
          | ty::PredicateKind::ObjectSafe(_)
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
}
