//! Thread local storage for storing data processed in rustc.
use std::cell::RefCell;

use index_vec::IndexVec;
use rustc_data_structures::fx::{
  FxHashSet as HashSet, FxIndexMap as IMap,
};
use rustc_hir::{def_id::LocalDefId};
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};

use rustc_span::Span;

pub use unsafe_tls::{
  store as unsafe_store_data, take as unsafe_take_data, FullObligationData,
  UODIdx,
};


use crate::{
  analysis::Provenance,
  types::{
    Obligation, ObligationHash,
  },
};

pub type RawTraitErrInfo =
  IMap<Span, (serde_json::Value, HashSet<Provenance<ObligationHash>>)>;

// NOTE: we use thread local storage to accumulate obligations
// accross call to the obligation inspector in `typeck_inspect`.
// DO NOT set this directly, make sure to use the function `push_obligaion`.
//
// TODO: documentation
thread_local! {
  static OBLIGATIONS: RefCell<Vec<Provenance<Obligation>>> = Default::default();

  static TREE: RefCell<Option<serde_json::Value>> = Default::default();

  static TRAIT_ERRORS: RefCell<RawTraitErrInfo> = Default::default();
}

// This is for complex obligations and their inference contexts.
// We don't want to store the entire inference context and obligation for
// every query, so we do it sparingly, things that make the cut:
// - "necessary" obligations, essentially trait predicates that don't
//   don't have a solution, and don't involve built-in traits or simple
//   types (e.g., unit `()`).
mod unsafe_tls {
  use super::*;
  use crate::analysis::EvaluationResult;

  thread_local! {
    static OBLIGATION_DATA: RefCell<IndexVec<UODIdx, Option<FullObligationData<'static>>>> =
      Default::default();
  }

  index_vec::define_index_type! {
    pub struct UODIdx = usize;
  }

  pub struct FullObligationData<'tcx> {
    pub infcx: InferCtxt<'tcx>,
    pub obligation: PredicateObligation<'tcx>,
    pub result: EvaluationResult,
  }

  pub fn store<'tcx>(
    infer_ctxt: &InferCtxt<'tcx>,
    obligation: &PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> UODIdx {
    OBLIGATION_DATA.with(|data| {
      let infcx = infer_ctxt.fork();
      let obl = obligation.clone();

      let infcx: InferCtxt<'static> = unsafe { std::mem::transmute(infcx) };
      let obligation: PredicateObligation<'static> =
        unsafe { std::mem::transmute(obl) };

      data.borrow_mut().push(Some(FullObligationData {
        infcx,
        obligation,
        result,
      }))
    })
  }

  pub fn take<'tcx>(idx: UODIdx) -> Option<FullObligationData<'tcx>> {
    OBLIGATION_DATA.with(|data| {
      let udata = data.borrow_mut()[idx].take()?;
      let data: FullObligationData<'tcx> =
        unsafe { std::mem::transmute(udata) };
      Some(data)
    })
  }
}

// ------------------------------------------------
// Obligation processing functions

/// Clone the inference context and obligation for later use.
///
/// NOTE: that this is an expensive operation, and should only be done
/// when necessary, an ultimate goal should be to get rid of it entirely.
pub fn store_obligation(_ldef_id: LocalDefId, obl: Provenance<Obligation>) {
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

pub fn take_obligations() -> Vec<Provenance<Obligation>> {
  OBLIGATIONS.with(|obls| obls.take())
}

// ------------------------------------------------
// Trait error processing functions

pub fn maybe_add_trait_error(
  span: Span,
  predicate: serde_json::Value,
  o: Provenance<ObligationHash>,
) {
  TRAIT_ERRORS.with(|errs| {
    log::debug!("CURRENT TRAIT ERRORS: {:#?}", errs.borrow());

    if let Some((_, ref mut errs)) = errs.borrow_mut().get_mut(&span) {
      errs.insert(o);
      return;
    }

    errs
      .borrow_mut()
      .insert(span, (predicate, Default::default()));
  })
}

pub fn take_trait_error_info() -> RawTraitErrInfo {
  TRAIT_ERRORS.with(|errs| errs.take())
}

// ------------------------------------------------
// Tree processing functions

pub fn store_tree(_json: serde_json::Value) {
  todo!()
  // TREE.with(|tree| {
  //   let prev = tree.replace(Some(json));
  //   debug_assert!(prev.is_none(), "replaced proof tree");
  // })
}

pub fn take_tree() -> Option<serde_json::Value> {
  todo!()
  // TREE.with(|tree| tree.take())
}
