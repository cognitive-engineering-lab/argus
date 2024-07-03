use std::collections::HashMap; // FIXME: change back to above
use std::{
  cell::RefCell,
  cmp::{Eq, PartialEq},
  hash::Hash,
};

use index_vec::{Idx, IndexVec};
// use rustc_data_structures::fx::FxHashMap as HashMap;
use rustc_middle::ty;

crate::define_idx! {
  usize,
  TyIdx
}

pub type TyInterner<'tcx> =
  RefCell<Interner<ty::Ty<'tcx>, TyIdx, serde_json::Value>>;

pub struct Interner<K: PartialEq + Eq + Hash, I: Idx, D> {
  values: IndexVec<I, D>,
  keys: HashMap<K, I>,
}

impl<K, I, D> Default for Interner<K, I, D>
where
  K: PartialEq + Eq + Hash,
  I: Idx,
{
  fn default() -> Self {
    Self {
      values: IndexVec::with_capacity(1_000_000),
      keys: HashMap::with_capacity(1_000_000),
    }
  }
}

impl<K, I, D> Interner<K, I, D>
where
  K: PartialEq + Eq + Hash,
  I: Idx,
{
  pub fn get_idx(&self, key: &K) -> Option<I> {
    self.keys.get(key).copied()
  }

  pub fn get_data(&self, key: &I) -> Option<&D> {
    self.values.get(*key)
  }

  pub fn insert(&mut self, k: K, d: D) -> I {
    let idx = self.values.push(d);
    self.keys.insert(k, idx);
    idx
  }

  pub fn insert_no_key(&mut self, d: D) -> I {
    self.values.push(d)
  }

  pub fn consume(self) -> IndexVec<I, D> {
    self.values
  }
}
