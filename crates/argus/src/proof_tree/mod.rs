pub mod topology;
pub mod ext;

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
    pub error_leaves: Vec<ProofNodeIdx>,
    pub topology: TreeTopology<ProofNodeIdx>,
}
