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

use super::ProofNodeIdx;

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
// more convenient to use `ProofNodeIdx` for ts-rs.
#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct TreeTopology {
  pub children: HashMap<ProofNodeIdx, HashSet<ProofNodeIdx>>,
  pub parent: HashMap<ProofNodeIdx, ProofNodeIdx>,
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

impl From<Path<ProofNodeIdx, ToRoot>> for super::ProofCycle {
  fn from(val: Path<ProofNodeIdx, ToRoot>) -> super::ProofCycle {
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

impl TreeTopology {
  pub fn new() -> Self {
    Self {
      children: HashMap::default(),
      parent: HashMap::default(),
    }
  }

  pub fn add(&mut self, from: ProofNodeIdx, to: ProofNodeIdx) {
    self.children.entry(from).or_default().insert(to);
    self.parent.insert(to, from);
  }

  pub fn is_parent(&self, parent: ProofNodeIdx, child: ProofNodeIdx) -> bool {
    self.parent.get(&child).is_some_and(|p| *p == parent)
  }

  pub fn is_leaf(&self, node: ProofNodeIdx) -> bool {
    match self.children.get(&node) {
      None => true,
      Some(children) => children.is_empty(),
    }
  }

  pub fn parent(&self, to: ProofNodeIdx) -> Option<ProofNodeIdx> {
    self.parent.get(&to).copied()
  }

  pub fn children(
    &self,
    from: ProofNodeIdx,
  ) -> impl Iterator<Item = ProofNodeIdx> + '_ {
    self
      .children
      .get(&from)
      .into_iter()
      .flat_map(|c| c.iter().copied())
  }

  pub fn iter(&self) -> impl Iterator<Item = ProofNodeIdx> + '_ {
    use itertools::Itertools;
    // TODO: just take the parents and chain the root
    self
      .parent
      .keys()
      .copied()
      .chain(self.children.keys().copied())
      .unique()
  }

  pub fn path_to_root(&self, node: ProofNodeIdx) -> Path<ProofNodeIdx, ToRoot> {
    let mut root = node;
    let mut curr = Some(node);
    let path = std::iter::from_fn(move || {
      let rootp = &mut root;
      let prev = curr;
      if let Some(n) = curr {
        curr = self.parent(n);
        *rootp = n;
      }

      prev
    });
    let path = path.collect::<Vec<_>>();

    Path {
      root,
      node,
      path,
      _marker: PhantomData,
    }
  }

  pub fn depth(&self, mut idx: ProofNodeIdx) -> usize {
    let mut d = 0;
    while let Some(p) = self.parent(idx) {
      d += 1;
      idx = p;
    }
    d
  }
}
