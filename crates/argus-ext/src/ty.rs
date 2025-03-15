mod r#impl;

use itertools::Itertools;
use rustc_data_structures::{fx::FxHashMap as HashMap, stable_hasher::Hash64};
use rustc_hir::{def_id::DefId, BodyId, HirId};
use rustc_infer::{infer::InferCtxt, traits::ObligationInspector};
use rustc_middle::ty::{self, Predicate, TyCtxt, TypeVisitable, TypeckResults};
use rustc_span::{FileName, Span};
use rustc_trait_selection::solve::inspect::InspectCandidate;
use rustc_utils::source_map::range::CharRange;
use smallvec::SmallVec;

use crate::EvaluationResult;

pub trait ImplCandidateExt<'tcx> {
  fn is_inductive(&self, tcx: TyCtxt<'tcx>) -> bool;
}

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

  fn is_alias(&self) -> bool;

  fn base_ty(&self) -> ty::Ty<'tcx>;
}

pub trait TyCtxtExt<'tcx> {
  fn body_filename(&self, body_id: BodyId) -> FileName;

  fn to_local(&self, body_id: BodyId, span: Span) -> Span;

  fn inspect_typeck(
    self,
    body_id: BodyId,
    inspector: ObligationInspector<'tcx>,
  ) -> &'tcx TypeckResults<'tcx>;

  /// Test whether `a` is a parent node of `b`.
  fn is_parent_of(&self, a: HirId, b: HirId) -> bool;

  fn predicate_hash(&self, p: &Predicate<'tcx>) -> Hash64;

  fn is_annotated_do_not_recommend(
    &self,
    candidate: &InspectCandidate<'_, 'tcx>,
  ) -> bool;

  fn does_trait_ref_occur_in(
    &self,
    needle: ty::PolyTraitRef<'tcx>,
    haystack: ty::Predicate<'tcx>,
  ) -> bool;

  fn function_arity(&self, ty: ty::Ty<'tcx>) -> Option<usize>;

  fn fn_trait_arity(&self, t: ty::TraitPredicate<'tcx>) -> Option<usize>;

  fn is_lang_item(&self, def_id: DefId) -> bool;
}

pub trait PredicateObligationExt {
  fn range(&self, tcx: &TyCtxt, body_id: BodyId) -> CharRange;
}

pub trait PredicateExt<'tcx> {
  fn expect_trait_predicate(&self) -> ty::PolyTraitPredicate<'tcx>;

  fn as_trait_predicate(&self) -> Option<ty::PolyTraitPredicate<'tcx>>;

  fn is_trait_predicate(&self) -> bool;

  fn is_lhs_unit(&self) -> bool;

  fn is_rhs_lang_item(&self, tcx: &TyCtxt) -> bool;

  fn is_trait_pred_rhs(&self, def_id: DefId) -> bool;

  fn is_main_ty_var(&self) -> bool;

  fn is_refined_by(&self, infcx: &InferCtxt<'tcx>, other: &Self) -> bool;
}

pub trait TypeckResultsExt<'tcx> {
  fn error_nodes(&self) -> impl Iterator<Item = HirId>;
}

pub trait VarCounterExt<'tcx>: TypeVisitable<TyCtxt<'tcx>> {
  fn count_vars(self, tcx: TyCtxt<'tcx>) -> usize;
}

fn make_failing_bound_implicationp<'a, 'tcx, T>(
  items: &'a [(usize, &T)],
  get_predicate: impl Fn(&T) -> Predicate<'tcx> + 'a,
  get_tcx: impl Fn(&T) -> TyCtxt<'tcx> + 'a,
) -> impl Fn((usize, &T)) -> bool + 'a {
  move |(i, other): (usize, &T)| {
    items.iter().any(|&(j, bound)| {
      let poly_tp = get_predicate(bound).expect_trait_predicate();
      if i != j // Don't consider reflexive implication
        && let ty::TraitPredicate {
          trait_ref,
          polarity: ty::PredicatePolarity::Positive,
        } = poly_tp.skip_binder()
      {
        get_tcx(other).does_trait_ref_occur_in(
          poly_tp.rebind(trait_ref),
          get_predicate(other),
        )
      } else {
        false
      }
    })
  }
}

/// If possible, use `retain_error_sources` which sorts and filters in place.
pub fn identify_error_sources<'tcx, T>(
  all_items: &[T],
  get_result: impl Fn(&T) -> EvaluationResult,
  get_predicate: impl Fn(&T) -> Predicate<'tcx>,
  get_tcx: impl Fn(&T) -> TyCtxt<'tcx>,
) -> impl Iterator<Item = usize> + '_ {
  let (trait_preds, _): (Vec<(usize, &T)>, _) =
    all_items.iter().enumerate().partition(|(_, t)| {
      !get_result(t).is_yes() && get_predicate(t).is_trait_predicate()
    });

  let is_implied_by_failing_bound =
    make_failing_bound_implicationp(&trait_preds, get_predicate, get_tcx);

  let mut to_keep = SmallVec::<[_; 8]>::default();
  for t in all_items.iter().enumerate() {
    if !is_implied_by_failing_bound(t) {
      to_keep.push(t.0);
    }
  }

  to_keep.into_iter()
}

/// Select only the items that are not implied by another failing bound.
///
/// ## Example:
///
/// 1. `Vec<T>: Foo (fail)`
/// 2. `<Vec<T> as Foo>::Assoc` (fail)
///
/// The second goal cannot succeed because the first didn't. The solver will
/// try to solve projection goals even if the base trait goal wasn't
/// successful. This function removes the implied goals (no matter the nesting depth).
#[must_use]
pub fn retain_error_sources<'tcx, T>(
  all_items: &mut [T],
  get_result: impl Fn(&T) -> EvaluationResult,
  get_predicate: impl Fn(&T) -> Predicate<'tcx>,
  get_tcx: impl Fn(&T) -> TyCtxt<'tcx>,
) -> usize {
  if all_items.is_empty() {
    return 0;
  }

  let idx = itertools::partition(&mut *all_items, |t| {
    !get_result(t).is_yes() && get_predicate(t).is_trait_predicate()
  });

  let (trait_preds, _) = all_items.split_at(idx);
  let trait_preds_enumerated =
    trait_preds.iter().enumerate().collect::<SmallVec<[_; 8]>>();

  let is_implied_by_failing_bound = make_failing_bound_implicationp(
    &trait_preds_enumerated,
    get_predicate,
    get_tcx,
  );

  let to_remove = &mut vec![];
  for t in all_items.iter().enumerate() {
    if is_implied_by_failing_bound(t) {
      to_remove.push(t.0);
    }
  }

  drop(is_implied_by_failing_bound);
  drop(trait_preds_enumerated);

  let mut swap_with = all_items.len();
  while let Some(i) = to_remove.pop() {
    swap_with -= 1;
    debug_assert!(swap_with < all_items.len());
    all_items.swap(i, swap_with);
  }

  swap_with
}

pub fn retain_method_calls<'tcx, T>(
  all_items: &mut Vec<T>,
  _get_result: impl Fn(&T) -> EvaluationResult,
  get_predicate: impl Fn(&T) -> Predicate<'tcx>,
  get_tcx: impl Fn(&T) -> TyCtxt<'tcx>,
) {
  if all_items.is_empty() {
    return;
  }

  let idx = itertools::partition(&mut *all_items, |t| {
    get_predicate(t).is_trait_predicate()
  });

  let (trait_preds, _) = all_items.split_at(idx);

  let mut grouped = HashMap::<_, Vec<_>>::default();
  for (i, t) in trait_preds.iter().enumerate() {
    let tp = get_predicate(t).expect_trait_predicate();
    let trait_id = tp.def_id();
    grouped.entry(trait_id).or_default().push(i);
  }

  let mut to_remove = vec![];
  let mut all_base_tys = vec![];
  let tcx = get_tcx(&all_items[0]);
  let deref_id = tcx.lang_items().deref_trait();

  for group in grouped.values() {
    let base_tys = group
      .iter()
      .map(|&i| {
        let tp = get_predicate(&trait_preds[i]).expect_trait_predicate();
        tp.self_ty().skip_binder().base_ty()
      })
      .unique();

    for base in base_tys.clone() {
      for &i in group {
        let tp = get_predicate(&trait_preds[i]).expect_trait_predicate();
        let here_ty = tp.self_ty().skip_binder();
        if here_ty != base && here_ty.base_ty() == base {
          to_remove.push(i);
        }
      }
    }
    all_base_tys.extend(base_tys);
  }

  // Remove all failed Deref attempts for the base types.
  if let Some(deref_id) = deref_id
    && let Some(deref_group) = grouped.get(&deref_id)
  {
    for deref_pred in deref_group {
      let tp =
        get_predicate(&trait_preds[*deref_pred]).expect_trait_predicate();
      let self_ty = tp.self_ty().skip_binder();
      if all_base_tys.iter().any(|&t| t == self_ty) {
        to_remove.push(*deref_pred);
      }
    }
  }

  to_remove.dedup();
  to_remove.sort_unstable();

  for i in to_remove.iter().rev() {
    all_items.remove(*i);
  }
}
