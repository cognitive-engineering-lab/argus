//! Thread local storage for storing data processed in rustc.
use std::cell::RefCell;

use index_vec::IndexVec;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
pub use unsafe_tls::{
  store as unsafe_store_data, take as unsafe_take_data, FullObligationData,
  SynIdx, UODIdx,
};

use crate::{
  proof_tree::SerializedTree,
  types::{intermediate::Provenance, Obligation},
};

// NOTE: we use thread local storage to accumulate obligations
// accross call to the obligation inspector in `typeck_inspect`.
// DO NOT set this directly, make sure to use the function `push_obligaion`.
//
// TODO: documentation
thread_local! {
  static OBLIGATIONS: RefCell<Vec<Provenance<Obligation>>> = Default::default();

  static TREE: RefCell<Option<SerializedTree>> = Default::default();
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

  index_vec::define_index_type! {
    pub struct UODIdx = usize;
  }
  index_vec::define_index_type! {
    pub struct SynIdx = usize;
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

// ------------------------------------------------
// Obligation processing functions

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

pub fn take_obligations() -> Vec<Provenance<Obligation>> {
  OBLIGATIONS.with(|obls| obls.take())
}

// ------------------------------------------------
// Tree processing functions

pub fn store_tree(new_tree: SerializedTree) {
  TREE.with(|tree| {
    let prev = tree.replace(Some(new_tree));
    log::warn!("replacing tree with {:?}", prev);
  })
}

pub fn take_tree() -> Option<SerializedTree> {
  TREE.with(|tree| tree.take())
}
