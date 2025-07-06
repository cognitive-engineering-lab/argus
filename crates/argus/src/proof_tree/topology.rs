//! Topology structures, mainly used by the `ProofTree`.

use std::{
  collections::{HashMap, HashSet},
  fmt::Debug,
  hash::Hash,
  marker::PhantomData,
};

use serde::Serialize;
#[cfg(feature = "testing")]
use ts_rs::TS;

use crate::proof_tree::ProofNode;

#[cfg(feature = "testing")]
pub trait Idx: Copy + PartialEq + Eq + Hash + Debug + Serialize + TS {}

#[cfg(feature = "testing")]
impl<T> Idx for T where T: Copy + PartialEq + Eq + Hash + Debug + Serialize + TS {}

#[cfg(not(feature = "testing"))]
pub trait Idx: Copy + PartialEq + Eq + Hash + Debug + Serialize {}

#[cfg(not(feature = "testing"))]
impl<T> Idx for T where T: Copy + PartialEq + Eq + Hash + Debug + Serialize {}

/// Parent child relationships between structures.
// NOTE: instead of using a generic parameter `I: Idx` it's
// more convenient to use `Node` for ts-rs.
#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct GraphTopology {
  pub children: HashMap<ProofNode, HashSet<ProofNode>>,
  pub parents: HashMap<ProofNode, HashSet<ProofNode>>,
}

#[derive(Clone, Debug)]
pub struct FromRoot;

#[derive(Clone, Debug)]
pub struct ToRoot;

/// The path from or to the root for a given node.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::struct_field_names)]
pub struct Path<N: Idx, Marker> {
  pub root: N,
  pub node: N,
  path: Vec<N>,
  _marker: PhantomData<Marker>,
}

impl<N: Idx, Marker> Path<N, Marker> {
  pub fn iter_inclusive(&self) -> impl Iterator<Item = &N> {
    self.path.iter()
  }

  pub fn iter_exclusive(&self) -> impl Iterator<Item = &N> {
    self.path.iter().skip(1)
  }

  pub fn len(&self) -> usize {
    self.path.len()
  }
}

impl<N: Idx> Path<N, ToRoot> {
  pub fn reverse(mut self) -> Path<N, FromRoot> {
    self.path.reverse();
    Path {
      root: self.root,
      node: self.node,
      path: self.path,
      _marker: PhantomData,
    }
  }
}

impl From<Path<ProofNode, ToRoot>> for super::ProofCycle {
  fn from(val: Path<ProofNode, ToRoot>) -> super::ProofCycle {
    let from_root = val.reverse();
    super::ProofCycle(from_root.path)
  }
}

impl<N: Idx> Path<N, FromRoot> {
  pub fn reverse(mut self) -> Path<N, ToRoot> {
    self.path.reverse();
    Path {
      root: self.root,
      node: self.node,
      path: self.path,
      _marker: PhantomData,
    }
  }
}

impl GraphTopology {
  pub fn new() -> Self {
    Self {
      children: HashMap::default(),
      parents: HashMap::default(),
    }
  }

  pub fn add(&mut self, from: ProofNode, to: ProofNode) {
    self.children.entry(from).or_default().insert(to);
    self.parents.entry(to).or_default().insert(from);
  }

  pub fn is_leaf(&self, node: ProofNode) -> bool {
    match self.children.get(&node) {
      None => true,
      Some(children) => children.is_empty(),
    }
  }

  pub fn children(
    &self,
    from: ProofNode,
  ) -> impl Iterator<Item = ProofNode> + '_ {
    self
      .children
      .get(&from)
      .into_iter()
      .flat_map(|c| c.iter().copied())
  }

  pub fn iter(&self) -> impl Iterator<Item = ProofNode> + '_ {
    use itertools::Itertools;
    // TODO: just take the parents and chain the root
    self
      .parents
      .keys()
      .copied()
      .chain(self.children.keys().copied())
      .unique()
  }
}
