//! Argus binding and interface types.
//!
//! NOTE: Auto-generated bindings aren't used currently due to deficiencies in
//! ts-rs functionality. This will hopefully catch up in the near future
//! so everything can once again be automatically generated. If *any*
//! binding or interface type is changed, this must be manually updated
//! in the [frontend](../../../ide/packages/common).
use std::{ops::Deref, str::FromStr};

use rustc_data_structures::stable_hasher::Hash64;
use rustc_middle::ty::{Predicate, TyCtxt};
use rustc_span::{symbol::Symbol, Span};

use anyhow::Result;
use rustc_utils::source_map::range::{CharRange, ToSpan};
use serde::{Serialize, Deserialize};

use crate::{proof_tree::Obligation, serialize::ty::{SymbolDef, PredicateDef}};

#[cfg(feature = "ts-rs")]
use ts_rs::TS;

#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct AmbiguityError<'tcx> {
    _marker: std::marker::PhantomData<&'tcx ()>
}

#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct TraitError<'tcx> {
  pub range: CharRange,
  pub candidates: Vec<ObligationHash>,
  #[serde(with = "PredicateDef")]
  #[cfg_attr(feature = "ts=rs", ts(type = "any"))]
  pub predicate: Predicate<'tcx>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct ObligationsInBody<'tcx> {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(serialize_with = "serialize_option")]
  #[cfg_attr(feature = "ts-rs", ts(type = "SymbolDef?"))]
  pub name: Option<Symbol>,
  pub range: CharRange,
  pub ambiguity_errors: Vec<AmbiguityError<'tcx>>,
  pub trait_errors: Vec<TraitError<'tcx>>,
  #[cfg_attr(feature = "ts-rs", ts(type = "Obligation[]"))]
  pub obligations: Vec<Obligation<'tcx>>,
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

#[derive(Deserialize, Serialize, Copy, Clone, Debug)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
pub struct ObligationHash(#[serde(with = "string")] u64);

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

#[derive(Debug)]
pub struct Target {
  pub hash: ObligationHash,
  pub span: Span,
}

pub trait ToTarget {
  fn to_target(self, tcx: TyCtxt) -> Result<Target>;
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
    use std::fmt::Display;
    use std::str::FromStr;

    use serde::{de, Serializer, Deserialize, Deserializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
        where T: Display,
              S: Serializer
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
        where T: FromStr,
              T::Err: Display,
              D: Deserializer<'de>
    {
        String::deserialize(deserializer)?.parse().map_err(de::Error::custom)
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
