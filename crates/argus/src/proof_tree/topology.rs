//! Topology structures, mainly used by the `ProofTree`.

use std::{
  collections::{HashMap, HashSet},
  fmt::Debug,
  hash::Hash,
  marker::PhantomData,
};

use serde::Serialize;
use ts_rs::TS;

use super::ProofNodeIdx;

pub trait Idx = Copy + PartialEq + Eq + Hash + Debug + Serialize + TS;

/// Parent child relationships between structures.
// NOTE: instead of using a generic parameter `I: Idx` it's
// more convenient to use `ProofNodeIdx` for ts-rs.
#[derive(TS, Serialize, Clone, Debug, Default, PartialEq, Eq)]
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

impl Into<super::ProofCycle> for Path<ProofNodeIdx, ToRoot> {
  fn into(self) -> super::ProofCycle {
    let from_root = self.reverse();
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

  pub fn is_leaf(&self, node: ProofNodeIdx) -> bool {
    match self.children.get(&node) {
      None => true,
      Some(children) => children.is_empty(),
    }
  }

  // pub fn children(
  //   &self,
  //   from: ProofNodeIdx,
  // ) -> impl Iterator<Item = ProofNodeIdx> {
  //   let v = match self.children.get(&from) {
  //     // Normally there are relatively few children.
  //     Some(kids) => kids
  //       .iter()
  //       .copied()
  //       .collect::<SmallVec<[ProofNodeIdx; 8]>>(),
  //     None => SmallVec::<[ProofNodeIdx; 8]>::default(),
  //   };

  //   v.into_iter()
  // }

  pub fn parent(&self, to: ProofNodeIdx) -> Option<ProofNodeIdx> {
    self.parent.get(&to).copied()
  }

  // pub fn is_parent_of(
  //   &self,
  //   parent: ProofNodeIdx,
  //   child: ProofNodeIdx,
  // ) -> bool {
  //   if let Some(p) = self.parent(child) {
  //     p == parent
  //   } else {
  //     false
  //   }
  // }

  // pub fn is_child_of(&self, child: ProofNodeIdx, parent: ProofNodeIdx) -> bool {
  //   self.is_parent_of(parent, child)
  // }

  // #[must_use]
  // pub fn add_in(&mut self, rhs: Self) -> Option<()> {
  //   let lhs_keys = self.children.keys().collect::<HashSet<_>>();
  //   for key in rhs.children.keys() {
  //     if lhs_keys.contains(key) {
  //       return None;
  //     }
  //   }

  //   self.children.extend(rhs.children.into_iter());
  //   self.parent.extend(rhs.parent.into_iter());

  //   Some(())
  // }

  // pub fn is_member(&self, node: ProofNodeIdx) -> bool {
  //   self
  //     .children
  //     .keys()
  //     .chain(self.parent.keys())
  //     .find(|&&n| n == node)
  //     .is_some()
  // }

  // pub fn leaves(&self) -> impl Iterator<Item = ProofNodeIdx> + '_ {
  //   self.parent.keys().filter(|n| self.is_leaf(**n)).copied()
  // }

  pub fn path_to_root(&self, node: ProofNodeIdx) -> Path<ProofNodeIdx, ToRoot> {
    let mut root = node;
    let mut curr = Some(node);
    let path = std::iter::from_fn(move || {
      let prev = curr;
      if let Some(n) = curr {
        curr = self.parent(n);
        root = n;
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
}
