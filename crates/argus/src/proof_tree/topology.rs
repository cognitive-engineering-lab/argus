//! Topology structures, mainly used by the `ProofTree`.

use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use std::{fmt::Debug, hash::Hash};

use serde::Serialize;

pub trait Idx = Copy + PartialEq + Eq + Hash + Serialize + Debug;

/// Parent child relationships between structures.
#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeTopology<I>
where
    I: Idx,
{
    pub children: FxHashMap<I, FxHashSet<I>>,
    pub parent: FxHashMap<I, I>,
}

pub trait HasTopology<I>
where
    I: Idx,
{
    fn get_topology(&self) -> &TreeTopology<I>;
}

impl<I: Idx> HasTopology<I> for TreeTopology<I> {
    fn get_topology(&self) -> &TreeTopology<I> {
        self
    }
}

impl<I: Idx> TreeTopology<I> {
    pub fn new() -> Self {
        Self {
            children: FxHashMap::default(),
            parent: FxHashMap::default(),
        }
    }

    pub fn add(&mut self, from: I, to: I) {
        self.children.entry(from).or_default().insert(to);
        self.parent.insert(to, from);
    }

    pub fn children(&self, from: I) -> impl Iterator<Item = I> {
        let v = match self.children.get(&from) {
            // Normally there are relatively few children.
            Some(kids) => kids.iter().copied().collect::<SmallVec<[I; 8]>>(),
            None => SmallVec::<[I; 8]>::default(),
        };

        v.into_iter()
    }

    pub fn parent(&self, to: I) -> Option<I> {
        self.parent.get(&to).copied()
    }

    pub fn is_parent_of(&self, parent: I, child: I) -> bool {
        if let Some(p) = self.parent(child) {
            p == parent
        } else {
            false
        }
    }

    pub fn is_child_of(&self, child: I, parent: I) -> bool {
        self.is_parent_of(parent, child)
    }

    pub fn convert<I2: Idx>(
        self,
        cvt_from: impl Fn(I) -> I2,
        cvt_to: impl Fn(I) -> I2,
    ) -> TreeTopology<I2> {
        let children = self
            .children
            .into_iter()
            .map(|(from, tos)| {
                let tos = tos
                    .into_iter()
                    .map(|to| cvt_to(to))
                    .collect::<FxHashSet<I2>>();
                (cvt_from(from), tos)
            })
            .collect::<FxHashMap<_, _>>();

        let parent = self
            .parent
            .into_iter()
            .map(|(to, from)| (cvt_to(to), cvt_from(from)))
            .collect::<FxHashMap<_, _>>();

        TreeTopology { children, parent }
    }

    #[must_use]
    pub fn add_in(&mut self, rhs: Self) -> Option<()> {
        let lhs_keys = self.children.keys().collect::<FxHashSet<_>>();
        for key in rhs.children.keys() {
            if lhs_keys.contains(key) {
                return None;
            }
        }

        self.children.extend(rhs.children.into_iter());
        self.parent.extend(rhs.parent.into_iter());

        Some(())
    }
}

impl<N: Idx> TreeTopology<N> {
    pub fn is_member(&self, node: N) -> bool {
        self.children
            .keys()
            .chain(self.parent.keys())
            .find(|&&n| n == node)
            .is_some()
    }

    pub fn is_leaf(&self, node: N) -> bool {
        match self.children.get(&node) {
            None => true,
            Some(children) => children.is_empty(),
        }
    }

    pub fn leaves(&self) -> impl Iterator<Item = N> + '_ {
        self.parent
            .keys()
            .filter(|n| self.is_leaf(**n))
            .copied()
    }

    pub fn map_from<U: Idx>(&self, start: N, func: impl FnMut(N) -> U) -> (U, TreeTopology<U>) {
        struct Dfs<'b, N: Idx, U: Idx> {
            new_topo: TreeTopology<U>,
            tree: &'b TreeTopology<N>,
        }

        impl<N: Idx, U: Idx> Dfs<'_, N, U> {
            fn dfs(&mut self, node: N, func: &mut dyn FnMut(N) -> U) -> U {
                let here = func(node);
                for child in self.tree.children(node) {
                    let childu = self.dfs(child, func);
                    self.new_topo.add(here, childu);
                }
                here
            }
        }

        let mut dfs = Dfs {
            new_topo: TreeTopology::new(),
            tree: self,
        };

        let mut closure = Box::new(func);
        let root = dfs.dfs(start, &mut *closure);

        (root, dfs.new_topo)
    }
}

trait TopologyVisitor<N: Idx> {
    fn get_topology(&self) -> &TreeTopology<N>;

    fn visit_node(&mut self, node: N);

    fn walk_node(&mut self, node: N) {
        let children = self.get_topology().children(node);
        for child in children {
            self.visit_node(child);
        }
    }
}
