//! Thread local storage for storing data processed in rustc.
use std::cell::RefCell;

use argus_ext::{
  infer::InferCtxtExt,
  ty::{EvaluationResultExt, PredicateExt},
};
use argus_ser::{
  self as ser,
  interner::{TyIdx, TyInterner},
};
use index_vec::IndexVec;
use rustc_data_structures::fx::FxIndexMap;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_span::Span;
pub use unsafe_tls::{
  access_interner as unsafe_access_interner, store as unsafe_store_data,
  take as unsafe_take_data, take_interned_values as take_interned_tys,
  FullObligationData, UODIdx,
};

use crate::{
  proof_tree::SerializedTree,
  types::{intermediate::Provenance, Obligation, ObligationHash},
};

const DRAIN_WINDOW: usize = 100;

// NOTE: we use thread local storage to accumulate obligations
// accross call to the obligation inspector in `typeck_inspect`.
// DO NOT set this directly, make sure to use the function `push_obligaion`.
//
// TODO: documentation
thread_local! {
  static BODY_DEF_PATH: RefCell<Option<serde_json::Value>> = RefCell::default();

  static OBLIGATIONS: RefCell<Vec<Provenance<Obligation>>> = RefCell::default();

  static TREE: RefCell<Option<SerializedTree>> = RefCell::default();

  static REPORTED_ERRORS: RefCell<FxIndexMap<Span, Vec<ObligationHash>>> = RefCell::default();
}

pub fn store_obligation(obl: Provenance<Obligation>) {
  OBLIGATIONS.with(|obls| {
    if !obls.borrow().iter().any(|o| *o.hash == *obl.hash) {
      obls.borrow_mut().push(obl);
    }
  });
}

// TODO: using `infcx` for error implication panics, but using
// stored contexts doesn't. Investigate why, as this certainly
// isn't a "solution."
pub fn drain_implied_ambiguities<'tcx>(
  _infcx: &InferCtxt<'tcx>,
  obligation: &PredicateObligation<'tcx>,
) {
  OBLIGATIONS.with(|obls| {
    let mut obls = obls.borrow_mut();

    let mut set = Vec::new();

    let lower_bound = obls.len().saturating_sub(DRAIN_WINDOW);
    let upper_bound = obls.len();

    for i in lower_bound .. upper_bound {
      let provenance = &obls[i];

      // Drain all elements that are:
      // 1. Ambiguous and--
      // 2. Implied by the passed obligation
      let is_ambig = provenance.result.is_maybe();
      let is_implied = provenance.full_data.map_or(false, |idx| {
        unsafe_tls::borrow_in(idx, |fdata| {
          let infcx = &fdata.infcx;
          let previous_pred = &fdata.obligation.predicate;
          let passing_pred = &obligation.predicate;
          previous_pred.is_refined_by(infcx, passing_pred)
        })
      });

      if is_ambig && is_implied {
        set.push(i);
      }
    }

    // TODO: we can make this faster by swapping elements to the end
    // then truncating the vector. Except that shuffles the order, which
    // we kind of rely on right now.
    for i in set.into_iter().rev() {
      obls.remove(i);
    }
  });
}

pub fn take_obligations() -> Vec<Provenance<Obligation>> {
  OBLIGATIONS.with(RefCell::take)
}

pub fn replace_reported_errors(infcx: &InferCtxt) {
  REPORTED_ERRORS.with(|rerrs| {
    if infcx.reported_trait_errors.borrow().len() == rerrs.borrow().len() {
      return;
    }

    let hashed_error_tree = infcx
      .reported_trait_errors
      .borrow()
      .iter()
      .map(|(span, (predicates, _))| {
        (
          *span,
          predicates
            .iter()
            .map(|p| infcx.predicate_hash(p).into())
            .collect::<Vec<_>>(),
        )
      })
      .collect::<FxIndexMap<_, _>>();

    rerrs.replace(hashed_error_tree);
  });
}

pub fn take_reported_errors() -> FxIndexMap<Span, Vec<ObligationHash>> {
  REPORTED_ERRORS.with(RefCell::take)
}

pub fn store_tree(new_tree: SerializedTree) {
  TREE.with(|tree| {
    if tree.borrow().is_none() {
      tree.replace(Some(new_tree));
    }
  });
}

pub fn take_tree() -> Option<SerializedTree> {
  TREE.with(RefCell::take)
}

// This is for complex obligations and their inference contexts.
// We don't want to store the entire inference context and obligation for
// every query, so we do it sparingly.
mod unsafe_tls {
  use super::*;
  use crate::analysis::EvaluationResult;

  thread_local! {
    static OBLIGATION_DATA: RefCell<IndexVec<UODIdx, Option<FullObligationData<'static>>>> =
      RefCell::default();

    static TY_INTERNER: TyInterner<'static> = TyInterner::default();
  }

  ser::define_idx! {
    usize,
    UODIdx
  }

  pub struct FullObligationData<'tcx> {
    pub infcx: InferCtxt<'tcx>,
    pub hash: ObligationHash,
    pub obligation: PredicateObligation<'tcx>,
    pub result: EvaluationResult,
  }

  impl PartialEq for FullObligationData<'_> {
    fn eq(&self, other: &Self) -> bool {
      self.infcx.universe() == other.infcx.universe()
        && self.hash == other.hash
        && self.obligation == other.obligation
        && self.result == other.result
    }
  }

  pub fn store<'tcx>(
    infer_ctxt: &InferCtxt<'tcx>,
    obligation: &PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> UODIdx {
    OBLIGATION_DATA.with(|data| {
      let infcx = infer_ctxt.fork();
      let obl = obligation.clone();
      let hash = infcx.predicate_hash(&obligation.predicate).into();

      let infcx: InferCtxt<'static> = unsafe { std::mem::transmute(infcx) };
      let obligation: PredicateObligation<'static> =
        unsafe { std::mem::transmute(obl) };

      data.borrow_mut().push(Some(FullObligationData {
        infcx,
        hash,
        obligation,
        result,
      }))
    })
  }

  // NOTE: ignore the 'tcx lifetime on the resulting reference. This data
  // lives as long as the thread does, but the function can only be used
  // from within this module so it shouldn't be an issue.
  pub(super) fn borrow_in<'tcx, R>(
    idx: UODIdx,
    f: impl FnOnce(&'tcx FullObligationData<'tcx>) -> R,
  ) -> R {
    OBLIGATION_DATA.with(|data| {
      let data = data.borrow();
      let ud = data.get(idx);
      let d: &'tcx FullObligationData<'tcx> =
        unsafe { std::mem::transmute(ud) };
      f(d)
    })
  }

  pub fn take<'tcx>() -> IndexVec<UODIdx, FullObligationData<'tcx>> {
    OBLIGATION_DATA.with(|data| {
      data
        .take()
        .into_iter()
        .map(|udata| {
          let data: FullObligationData<'tcx> =
            unsafe { std::mem::transmute(udata) };
          data
        })
        .collect()
    })
  }

  pub fn access_interner<'tcx, T>(
    f: impl for<'a> FnOnce(&'a TyInterner<'tcx>) -> T,
  ) -> T {
    TY_INTERNER.with(|interner: &TyInterner<'static>| {
      let interner: &TyInterner<'tcx> =
        unsafe { std::mem::transmute(interner) };
      f(interner)
    })
  }

  pub fn take_interned_values() -> IndexVec<TyIdx, serde_json::Value> {
    TY_INTERNER.with(RefCell::take).consume()
  }
}
