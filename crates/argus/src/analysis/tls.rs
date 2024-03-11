//! Thread local storage for storing data processed in rustc.
use std::{cell::RefCell, panic};

use index_vec::IndexVec;
use rustc_data_structures::fx::FxIndexMap;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_span::Span;
pub use unsafe_tls::{
  store as unsafe_store_data, take as unsafe_take_data, FullObligationData,
  SynIdx, UODIdx,
};

use crate::{
  ext::{EvaluationResultExt, InferCtxtExt},
  proof_tree::SerializedTree,
  rustc::InferCtxtExt as RustcInferCtxtExt,
  types::{intermediate::Provenance, Obligation, ObligationHash},
};

// NOTE: we use thread local storage to accumulate obligations
// accross call to the obligation inspector in `typeck_inspect`.
// DO NOT set this directly, make sure to use the function `push_obligaion`.
//
// TODO: documentation
thread_local! {
  static BODY_DEF_PATH: RefCell<Option<serde_json::Value>> = Default::default();

  static OBLIGATIONS: RefCell<Vec<Provenance<Obligation>>> = Default::default();

  static TREE: RefCell<Option<SerializedTree>> = Default::default();

  static REPORTED_ERRORS: RefCell<FxIndexMap<Span, Vec<ObligationHash>>> = Default::default();
}

pub fn store_obligation(obl: Provenance<Obligation>) {
  OBLIGATIONS.with(|obls| {
    if obls
      .borrow()
      .iter()
      .find(|o| *o.hash == *obl.hash)
      .is_none()
    {
      obls.borrow_mut().push(obl)
    }
  })
}

// TODO: using `infcx` for error implication panics, but using
// stored contexts doesn't. Investigate why, as this certainly
// isn't a "solution."
pub fn drain_implied_ambiguities<'tcx>(
  _infcx: &InferCtxt<'tcx>,
  obligation: &PredicateObligation<'tcx>,
) {
  use std::panic::AssertUnwindSafe;
  OBLIGATIONS.with(|obls| {
    let mut obls = obls.borrow_mut();
    obls.retain(|provenance| {
      // Drain all elements that are:
      // 1. Ambiguous, and--
      // 2. Implied by the passed obligation
      let should_remove = provenance.result.is_maybe()
        && provenance
          .full_data
          .map(|idx| {
            unsafe_tls::borrow_in(idx, |fdata| {
              // NOTE: using the inference context in this was is problematic as
              // we can't know for sure whether variables won't be leaked. (I.e.,
              // used in a context they don't live in.) The open snapshot check is
              // trying to mitigate this happening, but it's not foolproof and we
              // certainly don't want to crash the program.
              // Furthermore, this closure is unwind safe because the inference contexts
              // are forked, and no one outside this thread can access them.
              panic::catch_unwind(AssertUnwindSafe(|| {
                fdata.infcx.error_implies(
                  obligation.predicate.clone(),
                  fdata.obligation.predicate.clone(),
                )
              }))
              .unwrap_or(false)
            })
          })
          .unwrap_or(false);
      !should_remove
    })
  })
}

pub fn take_obligations() -> Vec<Provenance<Obligation>> {
  OBLIGATIONS.with(|obls| obls.take())
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
      .map(|(span, predicates)| {
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
  REPORTED_ERRORS.with(|rerrs| rerrs.take())
}

pub fn store_tree(new_tree: SerializedTree) {
  TREE.with(|tree| {
    if tree.borrow().is_none() {
      tree.replace(Some(new_tree));
    }
  })
}

pub fn take_tree() -> Option<SerializedTree> {
  TREE.with(|tree| tree.take())
}

// This is for complex obligations and their inference contexts.
// We don't want to store the entire inference context and obligation for
// every query, so we do it sparingly.
mod unsafe_tls {
  use super::*;
  use crate::analysis::EvaluationResult;

  thread_local! {
    static OBLIGATION_DATA: RefCell<IndexVec<UODIdx, Option<FullObligationData<'static>>>> =
      Default::default();
  }

  crate::define_idx! {
    usize,
    UODIdx,
    SynIdx
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
}
