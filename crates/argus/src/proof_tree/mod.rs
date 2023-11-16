pub mod topology;
pub mod ext;
pub mod pretty;
pub(super) mod serialize;

use rustc_data_structures::fx::FxHashSet as HashSet;

use topology::*;

use index_vec::IndexVec;
use serde::Serialize;

index_vec::define_index_type! {
    pub struct ProofNodeIdx = usize;
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct TreeDescription {
    /// The root node key.
    pub root: ProofNodeIdx,

    /// The leaf solution key.
    pub leaf: ProofNodeIdx,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct SerializedTree {
    pub descr: TreeDescription,
    pub nodes: IndexVec<ProofNodeIdx, String>,
    pub topology: TreeTopology<ProofNodeIdx>,
    pub error_leaves: Vec<ProofNodeIdx>,
    pub unnecessary_roots: HashSet<ProofNodeIdx>,
}
