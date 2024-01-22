//! Thread local storage for storing data processed in rustc.
use std::cell::RefCell;

use rustc_data_structures::fx::{
  FxHashMap as HashMap, FxHashSet as HashSet, FxIndexMap as IMap,
};
use rustc_hir::{def_id::LocalDefId, hir_id::HirId, BodyId};
use rustc_infer::infer::InferCtxt;
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;
use rustc_utils::source_map::range::CharRange;

use crate::{
  analysis::Provenance,
  ext::InferCtxtExt,
  types::{
    AmbiguityError, Obligation, ObligationHash, ObligationsInBody, TraitError,
  },
};

// NOTE: we use thread local storage to accumulate obligations
// accross call to the obligation inspector in `typeck_inspect`.
// DO NOT set this directly, make sure to use the function `push_obligaion`.
//
// TODO: documentation
thread_local! {
  static OBLIGATIONS: RefCell<HashMap<ObligationHash, Provenance<Obligation>>> = Default::default();

  static TREE: RefCell<Option<serde_json::Value>> = Default::default();

  // Map< Span -> (Predicate, Vec<Candidate>) >
  static TRAIT_ERRORS: RefCell<IMap<Span, (serde_json::Value, HashSet<Provenance<ObligationHash>>)>> = Default::default();

  static AMBIG_ERRORS: RefCell<IMap<Span, HashSet<Provenance<ObligationHash>>>> = Default::default();
}

// ------------------------------------------------
// Obligation processing functions

/// Store an obligation obtained from rustc.
pub fn store_obligation(_ldef_id: LocalDefId, obl: Provenance<Obligation>) {
  OBLIGATIONS.with(|obls| {
    let hash = obl.hash;
    let old = obls.borrow_mut().insert(hash, obl);
    assert!(old.is_none())
  })
}

pub fn take_obligations() -> HashMap<ObligationHash, Provenance<Obligation>> {
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

pub fn assemble_trait_errors(tcx: &TyCtxt) -> Vec<TraitError> {
  TRAIT_ERRORS.with(|errs| {
    let source_map = tcx.sess.source_map();
    errs
      .take()
      .into_iter()
      .map(|(span, (pv, cans))| {
        let range = CharRange::from_span(span, source_map)
          .expect("couldn't get trait error range");
        let candidates =
          cans.into_iter().map(|p| p.forget()).collect::<Vec<_>>();
        TraitError {
          range,
          candidates,
          predicate: pv,
        }
      })
      .collect::<Vec<_>>()
  })
}

// ------------------------------------------------
// Tree processing functions

pub fn store_tree(json: serde_json::Value) {
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
