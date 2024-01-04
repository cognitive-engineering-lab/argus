pub mod topology;
pub mod ext;
#[macro_use]
mod macros;
pub(super) mod serialize;

use std::collections::HashSet;

use index_vec::IndexVec;
use rustc_infer::traits::FulfilledObligation;
use rustc_trait_selection::traits::solve::Goal;
use rustc_middle::ty::{Predicate, Ty, TraitRef};
use rustc_utils::source_map::range::CharRange;

pub use topology::*;
use crate::ty::{PredicateDef, goal__predicate_def, my_ty::TyDef, TraitRefPrintOnlyTraitPathDef};

// FIXME: TS bindings were removed as the automatic 
// generation doesn't have a serde::remote-like feature.
use ts_rs::TS;
use serde::Serialize;

crate::define_usize_idx! {
  ProofNodeIdx
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(featutre = "ts-rs", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Node<'tcx> {
    Result { data: String },
    Candidate(Candidate<'tcx>),
    Goal {
      #[cfg_attr(featutre = "ts-rs", ts(type = "any"))]
      data: serde_json::Value,
      #[serde(skip)]
      _marker: std::marker::PhantomData<&'tcx ()>,
    },
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub enum Candidate<'tcx> {
  Impl {
    #[serde(with = "TyDef")]
    ty: Ty<'tcx>,
    #[serde(with = "TraitRefPrintOnlyTraitPathDef")]
    trait_ref: TraitRef<'tcx>,
    // range: CharRange,
  },
  Any(String), // TODO(gavinleroy) remove
}

impl From<&'static str> for Candidate<'_> {
  fn from(value: &'static str) -> Self {
    Candidate::Any(value.to_string())
  }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct SerializedTree<'tcx> {
    pub root: ProofNodeIdx,
    pub nodes: IndexVec<ProofNodeIdx, Node<'tcx>>,
    pub topology: TreeTopology<ProofNodeIdx>,
    pub error_leaves: Vec<ProofNodeIdx>,
    pub unnecessary_roots: HashSet<ProofNodeIdx>,
}

#[derive(Serialize, Clone, Debug)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(tag = "type")]
pub struct Obligation<'tcx> {
  #[cfg_attr(feature = "ts-rs", ts(type = "any"))]
  #[serde(with = "PredicateDef")]
  pub data: Predicate<'tcx>,
  // NOTE: Hash64 but we pass it as a String because JavaScript
  // cannot handle the full range of 64 bit integers.
  pub hash: String,
  pub range: CharRange,
  pub kind: ObligationKind,
}

#[derive(Serialize, Clone, Debug)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ObligationKind {
  Success,
  Failure,
}
