mod r#impl;

use rustc_data_structures::stable_hasher::Hash64;
use rustc_hir::{def_id::DefId, BodyId, HirId};
use rustc_infer::traits::ObligationInspector;
use rustc_middle::ty::{self, Predicate, TyCtxt, TypeVisitable, TypeckResults};
use rustc_span::{FileName, Span};
use rustc_trait_selection::solve::inspect::InspectCandidate;
use rustc_utils::source_map::range::CharRange;

use crate::EvaluationResult;

pub trait EvaluationResultExt {
  fn is_yes(&self) -> bool;

  fn is_maybe(&self) -> bool;

  fn is_no(&self) -> bool;

  fn is_better_than(&self, other: &EvaluationResult) -> bool;

  fn yes() -> Self;

  fn no() -> Self;

  fn maybe() -> Self;
}

pub trait TyExt<'tcx> {
  fn is_error(&self) -> bool;

  fn is_local(&self) -> bool;
}

pub trait TyCtxtExt<'tcx> {
  fn body_filename(&self, body_id: BodyId) -> FileName;

  fn to_local(&self, body_id: BodyId, span: Span) -> Span;

  fn inspect_typeck(
    self,
    body_id: BodyId,
    inspector: ObligationInspector<'tcx>,
  ) -> &TypeckResults;

  /// Test whether `a` is a parent node of `b`.
  fn is_parent_of(&self, a: HirId, b: HirId) -> bool;

  fn predicate_hash(&self, p: &Predicate<'tcx>) -> Hash64;

  fn is_annotated_do_not_recommend(
    &self,
    candidate: &InspectCandidate<'_, 'tcx>,
  ) -> bool;

  fn does_trait_ref_occur_in(
    &self,
    needle: ty::TraitRef<'tcx>,
    haystack: ty::Predicate<'tcx>,
  ) -> bool;

  fn function_arity(&self, ty: ty::Ty<'tcx>) -> Option<usize>;

  fn fn_trait_arity(&self, t: ty::TraitPredicate<'tcx>) -> Option<usize>;
}

pub trait PredicateObligationExt {
  fn range(&self, tcx: &TyCtxt, body_id: BodyId) -> CharRange;
}

pub trait PredicateExt<'tcx> {
  fn as_trait_predicate(&self) -> Option<ty::PolyTraitPredicate<'tcx>>;

  fn is_trait_predicate(&self) -> bool;

  fn is_lhs_unit(&self) -> bool;

  fn is_rhs_lang_item(&self, tcx: &TyCtxt) -> bool;

  fn is_trait_pred_rhs(&self, def_id: DefId) -> bool;

  fn is_main_ty_var(&self) -> bool;

  fn is_refined_by(&self, other: &Self) -> bool;
}

pub trait TypeckResultsExt<'tcx> {
  fn error_nodes(&self) -> impl Iterator<Item = HirId>;
}

pub trait VarCounterExt<'tcx>: TypeVisitable<TyCtxt<'tcx>> {
  fn count_vars(self, tcx: TyCtxt<'tcx>) -> usize;
}
