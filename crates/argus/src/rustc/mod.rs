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
    query::NoSolution, FulfillmentError,
    FulfillmentErrorCode, MismatchedProjectionTypes,
    PredicateObligation, SelectionError,
  },
};
use rustc_middle::{
  ty,
  ty::error::{ExpectedFound, TypeError},
};


use crate::types::intermediate::EvaluationResult;

pub mod fn_ctx;

macro_rules! bug {
  ($( $tree:tt ),*) => {
    panic!( $( $tree )* )
  }
}

pub trait InferCtxtExt<'tcx> {
  fn to_fulfillment_error(
    &self,
    obligation: &PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> Option<FulfillmentError<'tcx>>;
}

impl<'tcx> InferCtxtExt<'tcx> for InferCtxt<'tcx> {
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
        code: match goal.predicate.kind().skip_binder() {
          ty::PredicateKind::Clause(ty::ClauseKind::Projection(_)) => {
            FulfillmentErrorCode::ProjectionError(
              // FIXME: This could be a `Sorts` if the term is a type
              MismatchedProjectionTypes {
                err: TypeError::Mismatch,
              },
            )
          }
          ty::PredicateKind::NormalizesTo(..) => {
            FulfillmentErrorCode::ProjectionError(MismatchedProjectionTypes {
              err: TypeError::Mismatch,
            })
          }
          ty::PredicateKind::AliasRelate(_, _, _) => {
            FulfillmentErrorCode::ProjectionError(MismatchedProjectionTypes {
              err: TypeError::Mismatch,
            })
          }
          ty::PredicateKind::Subtype(pred) => {
            let (a, b) = infcx.instantiate_binder_with_placeholders(
              goal.predicate.kind().rebind((pred.a, pred.b)),
            );
            let expected_found = ExpectedFound::new(true, a, b);
            FulfillmentErrorCode::SubtypeError(
              expected_found,
              TypeError::Sorts(expected_found),
            )
          }
          ty::PredicateKind::Coerce(pred) => {
            let (a, b) = infcx.instantiate_binder_with_placeholders(
              goal.predicate.kind().rebind((pred.a, pred.b)),
            );
            let expected_found = ExpectedFound::new(false, a, b);
            FulfillmentErrorCode::SubtypeError(
              expected_found,
              TypeError::Sorts(expected_found),
            )
          }
          ty::PredicateKind::Clause(_)
          | ty::PredicateKind::ObjectSafe(_)
          | ty::PredicateKind::Ambiguous => {
            FulfillmentErrorCode::SelectionError(SelectionError::Unimplemented)
          }
          ty::PredicateKind::ConstEquate(..) => {
            bug!("unexpected goal: {goal:?}")
          }
        },
        root_obligation: obligation,
      },
    )
  }
}
