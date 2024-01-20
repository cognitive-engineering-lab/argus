//! Thread local storage for storing data processed in rustc.
use std::cell::RefCell;

use rustc_hir::def_id::LocalDefId;

use crate::types::{AmbiguityError, Obligation, TraitError};

// NOTE: we use thread local storage to accumulate obligations
// accross call to the obligation inspector in `typeck_inspect`.
// DO NOT set this directly, make sure to use the function `push_obligaion`.
thread_local! {
  /// TODO: documentation
  static OBLIGATIONS: RefCell<Vec<Obligation>> = Default::default();

  /// TODO: documentation
  static TREE: RefCell<Option<serde_json::Value>> = Default::default();
}

// ------------------------------------------------
// Obligation processing functions

/// Store an obligation obtained from rustc.
pub fn push_obligation(_ldef_id: LocalDefId, obl: Obligation) {
  OBLIGATIONS.with(|obls| {
    obls.borrow_mut().push(obl);
  })
}

/// Retrieve *all* obligations processed from rustc.
pub fn take_obligations() -> Vec<Obligation> {
  OBLIGATIONS.with(|obls| {
    // FIXME: this is a HACK to overcome the unimplemented
    // code in serialization.
    obls
      .take()
      .into_iter()
      .filter(|o| o.is_necessary)
      // .unique_by(|o| o.hash)
      .collect::<Vec<_>>()
  })
}

pub fn get_trait_errors() -> Vec<TraitError> {
  vec![]
}

pub fn get_ambiguity_errors() -> Vec<AmbiguityError> {
  vec![]
}

// ------------------------------------------------
// Tree processing functions

pub fn store_tree(json: serde_json::Value) {
  TREE.with(|tree| {
    let prev = tree.replace(Some(json));
    debug_assert!(prev.is_none(), "replaced proof tree");
  })
}

pub fn take_tree() -> Option<serde_json::Value> {
  TREE.with(|tree| tree.take())
}
