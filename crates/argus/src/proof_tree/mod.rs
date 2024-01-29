//! Proof tree types sent to the Argus frontend.

pub mod ext;
// FIXME: update topology module for current needs
#[allow(dead_code, unused_assignments, unused_variables)]
pub mod topology;
#[macro_use]
mod macros;
pub(super) mod serialize;

use std::collections::HashSet;

use index_vec::IndexVec;
use rustc_hir as hir;
use rustc_middle::ty::{TraitRef, Ty};
use serde::Serialize;
pub use topology::*;
#[cfg(feature = "ts-rs")]
use ts_rs::TS;

use crate::serialize::{
  hir::ImplDef,
  ty::{TraitRefPrintOnlyTraitPathDef, TyDef},
};

crate::define_usize_idx! {
  ProofNodeIdx
}

#[derive(Serialize, Debug, Clone)]
#[cfg_attr(featutre = "ts-rs", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Node<'tcx> {
  Result {
    data: String,
  },
  Candidate {
    data: Candidate<'tcx>,
  },
  Goal {
    #[cfg_attr(featutre = "ts-rs", ts(type = "any"))]
    data: serde_json::Value,

    #[serde(skip)]
    _marker: std::marker::PhantomData<&'tcx ()>,
  },
}

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Candidate<'hir> {
  Impl {
    #[serde(with = "ImplDef")]
    data: &'hir hir::Impl<'hir>,

    fallback: String,
  },

  // TODO(gavinleroy) when everything is structured
  Any {
    data: String,
  },
}

impl From<&'static str> for Candidate<'_> {
  fn from(value: &'static str) -> Self {
    Candidate::Any {
      data: value.to_string(),
    }
  }
}

#[derive(Serialize, Debug, Clone)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct SerializedTree<'tcx> {
  pub root: ProofNodeIdx,
  pub nodes: IndexVec<ProofNodeIdx, Node<'tcx>>,
  pub topology: TreeTopology<ProofNodeIdx>,
  pub error_leaves: Vec<ProofNodeIdx>,
  pub unnecessary_roots: HashSet<ProofNodeIdx>,
}
