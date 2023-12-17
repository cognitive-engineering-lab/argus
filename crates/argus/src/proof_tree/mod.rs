pub mod topology;
pub mod ext;
#[macro_use]
mod macros;
pub(super) mod serialize;

use std::collections::HashSet;

use index_vec::IndexVec;
use rustc_infer::traits::FulfilledObligation;
// use rustc_macros::RustcEncodable;
use rustc_middle::ty::Predicate;
use rustc_utils::source_map::range::CharRange;

pub use topology::*;
use crate::ty::PredicateDef;

use ts_rs::TS;
use serde::Serialize;

crate::define_usize_idx! {
  ProofNodeIdx
}

#[derive(TS, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Node {
    Result { data: String },
    Goal { data: String },
    Candidate { data: String },
}

#[derive(TS, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SerializedTree {
    pub root: ProofNodeIdx,
    pub nodes: IndexVec<ProofNodeIdx, Node>,
    pub topology: TreeTopology<ProofNodeIdx>,
    pub error_leaves: Vec<ProofNodeIdx>,
    pub unnecessary_roots: HashSet<ProofNodeIdx>,
}

#[derive(Serialize, Clone, Debug)]
// TS,
#[serde(tag = "type")]
pub struct Obligation<'tcx> {
  // #[ts(rename = "any")]
  #[serde(with = "PredicateDef")]
  pub data: Predicate<'tcx>,
  // NOTE: Hash64 but we pass it as a String because JavaScript
  // cannot handle the full range of 64 bit integers.
  pub hash: String,
  pub range: CharRange,
  pub kind: ObligationKind,
}

#[derive(TS, Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ObligationKind {
  Success,
  Failure,
}
