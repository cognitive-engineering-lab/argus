//! Argus binding and interface types.
//!
//! NOTE: (1) Auto-generated bindings aren't used currently due to deficiencies in
//! ts-rs functionality. This will hopefully catch up in the near future
//! so everything can once again be automatically generated. If *any*
//! binding or interface type is changed, this must be manually updated
//! in the [frontend](../../../ide/packages/common).
//!
//! NOTE: (2) All types in this file must *erase* values coming from
//! an inference context. This means any type serialization must be
//! done first and stored as a `serde_json::Value`. This erasure isn't
//! ideal but it's how we can currently use the `InferCtxt` used during
//! trait resolution.
use std::{ops::Deref, str::FromStr};

use rustc_data_structures::{
  stable_hasher::Hash64,
  fx::{FxHashMap as HashMap, FxHashSet as HashSet}
};
use rustc_middle::ty::{Predicate, TyCtxt};
use rustc_span::{symbol::Symbol, Span};


use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::traits::{solve::Certainty, query::NoSolution};


use index_vec::IndexVec;
use rustc_utils::source_map::range::{CharRange, ToSpan};
use serde::{Deserialize, Serialize};
use anyhow::Result;
#[cfg(feature = "ts-rs")]
use ts_rs::TS;

use crate::serialize::ty::{PredicateDef, SymbolDef};

// -----------------

type IdxSize = u32;

index_vec::define_index_type! {
    pub struct ObligationIdx = IdxSize;
}

index_vec::define_index_type! {
    pub struct TraitErrorIdx = IdxSize;
}

index_vec::define_index_type! {
    pub struct AmbiguousErrorIdx = IdxSize;
}

index_vec::define_index_type! {
    pub struct TyIdx = IdxSize;
}


#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct AmbiguityError {}


#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct TraitError {
  pub range: CharRange,
  pub candidates: Vec<ObligationHash>,
  #[cfg_attr(feature = "ts-rs", ts(type = "any"))]
  /// Actual type, `Predicate<'tcx>`
  pub predicate: serde_json::Value,
}


#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct ObligationsInBody {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(serialize_with = "serialize_option")]
  #[cfg_attr(feature = "ts-rs", ts(type = "SymbolDef | undefined"))]
  pub name: Option<Symbol>,

  /// Range of the represented body.
  pub range: CharRange,

  /// All ambiguous expression in the body. These *could* involve
  /// trait errors, so it's important that we can map the specific
  /// obligations to these locations. (That is, if they occur.)
  pub ambiguity_errors: Vec<AmbiguityError>,

  /// Concrete trait errors, this would be when the compiler
  /// can say for certainty that a specific trait bound was required
  /// but not satisfied.
  pub trait_errors: Vec<TraitError>,

  #[cfg_attr(feature = "ts-rs", ts(type = "Obligation[]"))]
  pub obligations: Vec<Obligation>,
}


/// The set of relations between obligation data.
// TODO: it may also be interesting to include information such as:
// - trait visibility, which traits are currently visible.
// - ...
#[derive(Serialize)]
pub struct Relations {

  /// Bounds errors : TraitError -> [Obligation]
  ///
  /// When a hard trait bound occurs, e.g., `Vec<(i32, i32)>: Clone`,
  /// there is 'generally' a single corresponding proof tree.
  /// `TraitErrors` are shown in the editor and we want to
  /// point users to the tree in the explorer window.
  bounds_map: HashMap<TraitErrorIdx, HashSet<ObligationIdx>>,

  /// Ambiguity : AmbiguityError -> [Obligation]
  ///
  /// An ambiguous expression may arise when a term such as
  /// `obj.frobnicate()`, resolving this happens in "three steps."
  ///
  /// 1. Find the monomorphized type of the variable `obj`. Let's
  ///    call this `T_0`.
  ///
  /// 2. Enumerate all visible traits `C_0, C_1, ..., C_m`
  ///    that contain a trait method `frobnicate`.
  ///
  /// 3. Find all pairs `(T_i, C_j)` such that `T_i: T_j`.
  ///    The types `T_0, T_1, ..., T_n` are such that
  ///    `∀ i. 0 ≤ i < n - 1 ⟹  *T_i == T_i+1`
  ///    (that is, `T_i` dereferences to `T_i+1`).
  ///
  /// Step 3 is generally where confusion creeps in.
  ambiguity_map: HashMap<TraitErrorIdx, HashSet<ObligationIdx>>,

  /// Derefs : Ty -> Ty
  ///
  /// This is important for knowing which obligations are sort of
  /// "nested" in others. For example, the two obligations
  ///
  /// - Vec<(i32, i32)>: Clone
  /// - &[(i32, i32)]: Clone
  ///
  /// Are related by the fact that `Vec<(i32, i32)>: Deref` and
  /// `<Vec<(i32, i32)> as Deref>::Target == &[(i32, i32)]`.
  deref_map: HashMap<TraitErrorIdx, ObligationIdx>,
}


#[derive(Serialize, Clone, Debug)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(tag = "type")]
pub struct Obligation {
  #[cfg_attr(feature = "ts-rs", ts(type = "any"))]
  /// Actual type: Predicate<'tcx>,
  pub predicate: serde_json::Value,
  pub hash: ObligationHash,
  pub range: CharRange,
  pub kind: ObligationKind,
  pub is_necessary: bool,
}


#[derive(Serialize, Clone, Debug)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ObligationKind {
  Success,
  Ambiguous,
  Failure,
}


// Serialize an Option<Symbol> using SymbolDef but the value must be a Some(..)
fn serialize_option<S: serde::Serializer>(
  value: &Option<Symbol>,
  s: S,
) -> Result<S::Ok, S::Error> {
  let Some(symb) = value else {
    unreachable!();
  };

  SymbolDef::serialize(symb, s)
}


#[derive(Deserialize, Serialize, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
pub struct ObligationHash(#[serde(with = "string")] u64);


#[derive(Debug)]
pub struct Target {
  pub hash: ObligationHash,
  pub span: Span,
}


pub trait ToTarget {
  fn to_target(self, tcx: TyCtxt) -> Result<Target>;
}


// ------------------------------


impl Deref for ObligationHash {
  type Target = u64;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}


impl FromStr for ObligationHash {
  type Err = anyhow::Error;
  fn from_str(s: &str) -> Result<Self> {
    Ok(<u64 as FromStr>::from_str(s)?.into())
  }
}


impl From<u64> for ObligationHash {
  fn from(value: u64) -> Self {
    ObligationHash(value)
  }
}

impl From<Hash64> for ObligationHash {
  fn from(value: Hash64) -> Self {
    value.as_u64().into()
  }
}



impl<U: Into<ObligationHash>, T: ToSpan> ToTarget for (U, T) {
  fn to_target(self, tcx: TyCtxt) -> Result<Target> {
    self.1.to_span(tcx).map(|span| Target {
      hash: self.0.into(),
      span,
    })
  }
}


mod string {
  use std::{fmt::Display, str::FromStr};

  use serde::{de, Deserialize, Deserializer, Serializer};

  pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
  where
    T: Display,
    S: Serializer,
  {
    serializer.collect_str(value)
  }

  pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
  where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
  {
    String::deserialize(deserializer)?
      .parse()
      .map_err(de::Error::custom)
  }
}


// Types that do not live past a single inspection run. Use these
// to build up intermediate information that *does not* need to
// be stored in TLS.
pub(super) mod intermediate {
  use super::*;

  pub type EvaluationResult = Result<Certainty, NoSolution>;

  pub struct FulfillmentData<'a, 'tcx: 'a> {
    pub hash: Hash64,
    pub obligation: &'a PredicateObligation<'tcx>,
    pub result: EvaluationResult,
  }

  impl FulfillmentData<'_, '_> {
    pub fn kind(&self) -> ObligationKind {
      match self.result {
        Ok(Certainty::Yes) => ObligationKind::Success,
        Ok(..) => ObligationKind::Ambiguous,
        Err(..) => ObligationKind::Failure,
      }
    }
  }
}

#[cfg(test)]
mod tests {

  macro_rules! ts {
      ($($ty:ty,)*) => {
        $({
          let error_msg = format!("Failed to export TS binding for type '{}'", stringify!($ty));
          <$ty as TS>::export().expect(error_msg.as_ref());
        })*
      };
    }

  #[test]
  fn export_bindings_all_tys() {
    ts! {
      // proof_tree::SerializedTree,
      // proof_tree::Node,
      // proof_tree::Obligation,
      // proof_tree::TreeTopology<proof_tree::ProofNodeIdx>,

      // From rustc_utils
      // range::CharRange,
      // range::CharPos,
      // filename::FilenameIndex,
    }
  }
}
