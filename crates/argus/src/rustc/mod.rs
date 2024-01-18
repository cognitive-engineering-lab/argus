//! Functionality coming from Rustc.
//!
//! These are things that we might be able to convince people to make
//! public within Rustc itself, but our needs may change so it hasn't
//! happened yet.

use rustc_data_structures::fx::FxIndexSet;
use rustc_hir_typeck::FnCtxt;
use rustc_infer::{
  infer::error_reporting::TypeErrCtxt, traits::util::elaborate,
};
use rustc_middle::ty::{self, ToPolyTraitRef};

// ------------------------
// Used interface
use crate::analysis::FulfillmentData;

pub trait FnCtxtExt<'tcx> {
  // NOTE: the errors taken are of `FulfillmentData` to conform to local needs
  fn adjust_fulfillment_errors_for_expr_obligation(
    &self,
    errors: &mut Vec<FulfillmentData<'_, 'tcx>>,
  );
}

pub trait InferCtxtExt<'tcx> {
  fn error_implies(
    &self,
    cond: ty::Predicate<'tcx>,
    error: ty::Predicate<'tcx>,
  ) -> bool;
}

// ------------------------
// Impls

impl<'tcx> FnCtxtExt<'tcx> for FnCtxt<'_, 'tcx> {
  fn adjust_fulfillment_errors_for_expr_obligation(
    &self,
    errors: &mut Vec<FulfillmentData<'_, 'tcx>>,
  ) {
    todo!()

    // let mut remap_cause = FxIndexSet::default();
    // let mut not_adjusted = vec![];

    // for fdata in errors {
    //   let FulfilledDataKind::Err(error) = &mut fdata.data else {
    //     continue;
    //   };

    //   let before_span = error.obligation.cause.span;
    //   if self.adjust_fulfillment_error_for_expr_obligation(error)
    //     || before_span != error.obligation.cause.span
    //   {
    //     remap_cause.insert((
    //       before_span,
    //       error.obligation.predicate,
    //       error.obligation.cause.clone(),
    //     ));
    //   } else {
    //     not_adjusted.push(error);
    //   }
    // }

    // for error in not_adjusted {
    //   for (span, predicate, cause) in &remap_cause {
    //     if *predicate == error.obligation.predicate
    //       && span.contains(error.obligation.cause.span)
    //     {
    //       error.obligation.cause = cause.clone();
    //       continue;
    //     }
    //   }
    // }
  }
}

// Taken from rustc_trait_selection/src/traits/error_reporting/type_err_ctxt_ext.rs
impl<'tcx> InferCtxtExt<'tcx> for TypeErrCtxt<'_, 'tcx> {
  fn error_implies(
    &self,
    cond: ty::Predicate<'tcx>,
    error: ty::Predicate<'tcx>,
  ) -> bool {
    use log::debug;

    if cond == error {
      return true;
    }

    // FIXME: It should be possible to deal with `ForAll` in a cleaner way.
    let bound_error = error.kind();
    let (cond, error) =
      match (cond.kind().skip_binder(), bound_error.skip_binder()) {
        (
          ty::PredicateKind::Clause(ty::ClauseKind::Trait(..)),
          ty::PredicateKind::Clause(ty::ClauseKind::Trait(error)),
        ) => (cond, bound_error.rebind(error)),
        _ => {
          // FIXME: make this work in other cases too.
          return false;
        }
      };

    for pred in elaborate(self.tcx, std::iter::once(cond)) {
      let bound_predicate = pred.kind();
      if let ty::PredicateKind::Clause(ty::ClauseKind::Trait(implication)) =
        bound_predicate.skip_binder()
      {
        let error = error.to_poly_trait_ref();
        let implication = bound_predicate.rebind(implication.trait_ref);
        // FIXME: I'm just not taking associated types at all here.
        // Eventually I'll need to implement param-env-aware
        // `Γ₁ ⊦ φ₁ => Γ₂ ⊦ φ₂` logic.
        let param_env = ty::ParamEnv::empty();
        if self.can_sub(param_env, error, implication) {
          debug!(
            "error_implies: {:?} -> {:?} -> {:?}",
            cond, error, implication
          );
          return true;
        }
      }
    }

    false
  }
}
