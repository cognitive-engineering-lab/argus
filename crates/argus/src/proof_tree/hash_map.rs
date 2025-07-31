//! Wrapper around `FxHashMap` that implements `ts_rs::TS`
use std::{
  fmt::Debug,
  ops::{Deref, DerefMut},
};

use rustc_data_structures::fx::FxHashMap;
use serde::Serialize;
#[cfg(feature = "testing")]
use ts_rs::TS;

use super::topology::Idx;

#[cfg(not(feature = "testing"))]
pub trait AllTheThings: Debug + Serialize {}

#[cfg(not(feature = "testing"))]
impl<T> AllTheThings for T where T: Debug + Serialize {}

#[cfg(feature = "testing")]
pub trait AllTheThings: Debug + Serialize + TS {}

#[cfg(feature = "testing")]
impl<T> AllTheThings for T where T: Debug + Serialize + TS {}

#[derive(Clone, Debug, Serialize)]
pub struct HashMap<K: Idx, V: AllTheThings>(FxHashMap<K, V>);

impl<K: Idx, V: AllTheThings> Deref for HashMap<K, V> {
  type Target = FxHashMap<K, V>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<K: Idx, V: AllTheThings> DerefMut for HashMap<K, V> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl<K: Idx, V: AllTheThings> Default for HashMap<K, V> {
  fn default() -> Self {
    HashMap(FxHashMap::default())
  }
}

#[cfg(feature = "testing")]
impl<K: Idx, V: AllTheThings> TS for HashMap<K, V> {
  fn name() -> String {
    "Record".to_owned()
  }

  fn name_with_type_args(args: Vec<String>) -> String {
    assert_eq!(
      args.len(),
      2,
      "called HashMap::name_with_type_args with {} args",
      args.len()
    );
    format!("Record<{}, {}>", args[0], args[1])
  }

  fn inline() -> String {
    format!("Record<{}, {}>", K::inline(), V::inline())
  }

  fn dependencies() -> Vec<ts_rs::Dependency>
  where
    Self: 'static,
  {
    [
      ts_rs::Dependency::from_ty::<K>(),
      ts_rs::Dependency::from_ty::<V>(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }

  fn transparent() -> bool {
    true
  }
}
