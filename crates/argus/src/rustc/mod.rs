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

  fn error_implies(
    &self,
    cond: ty::Predicate<'tcx>,
    error: ty::Predicate<'tcx>,
  ) -> bool;
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
            let (a, b) = infcx.enter_forall_and_leak_universe(
              goal.predicate.kind().rebind((pred.a, pred.b)),
            );
            let expected_found = ExpectedFound::new(true, a, b);
            FulfillmentErrorCode::SubtypeError(
              expected_found,
              TypeError::Sorts(expected_found),
            )
          }
          ty::PredicateKind::Coerce(pred) => {
            let (a, b) = infcx.enter_forall_and_leak_universe(
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

  fn error_implies(
    &self,
    cond: ty::Predicate<'tcx>,
    error: ty::Predicate<'tcx>,
  ) -> bool {
    use rustc_middle::ty::ToPolyTraitRef;
    use rustc_trait_selection::traits::elaborate;

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
          log::debug!(
            "error_implies: {:?} -> {:?} -> {:?}",
            cond,
            error,
            implication
          );
          return true;
        }
      }
    }

    false
  }
}

pub fn to_pretty_impl_header(
  tcx: ty::TyCtxt<'_>,
  impl_def_id: rustc_hir::def_id::DefId,
) -> Option<String> {
  use std::fmt::Write;

  use rustc_data_structures::fx::FxIndexSet;
  use ty::GenericArgs;

  let trait_ref = tcx.impl_trait_ref(impl_def_id)?.instantiate_identity();
  let mut w = "impl".to_owned();

  let args = GenericArgs::identity_for_item(tcx, impl_def_id);

  // FIXME: Currently only handles ?Sized.
  //        Needs to support ?Move and ?DynSized when they are implemented.
  let mut types_without_default_bounds = FxIndexSet::default();
  let sized_trait = tcx.lang_items().sized_trait();

  let arg_names = args
    .iter()
    .map(|k| k.to_string())
    .filter(|k| k != "'_")
    .collect::<Vec<_>>();
  if !arg_names.is_empty() {
    types_without_default_bounds.extend(args.types());
    w.push('<');
    w.push_str(&arg_names.join(", "));
    w.push('>');
  }

  write!(
    w,
    " {} for {}",
    trait_ref.print_only_trait_path(),
    tcx.type_of(impl_def_id).instantiate_identity()
  )
  .unwrap();

  // The predicates will contain default bounds like `T: Sized`. We need to
  // remove these bounds, and add `T: ?Sized` to any untouched type parameters.
  let predicates = tcx.predicates_of(impl_def_id).predicates;
  let mut pretty_predicates =
    Vec::with_capacity(predicates.len() + types_without_default_bounds.len());

  for (p, _) in predicates {
    if let Some(poly_trait_ref) = p.as_trait_clause() {
      if Some(poly_trait_ref.def_id()) == sized_trait {
        // FIXME(#120456) - is `swap_remove` correct?
        types_without_default_bounds
          .swap_remove(&poly_trait_ref.self_ty().skip_binder());
        continue;
      }
    }
    pretty_predicates.push(p.to_string());
  }

  pretty_predicates.extend(
    types_without_default_bounds
      .iter()
      .map(|ty| format!("{ty}: ?Sized")),
  );

  if !pretty_predicates.is_empty() {
    write!(w, "\n  where {}", pretty_predicates.join(", ")).unwrap();
  }

  w.push(';');
  Some(w)
}
