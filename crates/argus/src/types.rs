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

use anyhow::Result;
use index_vec::IndexVec;
use rustc_data_structures::{
  fx::{FxHashMap as HashMap, FxHashSet as HashSet},
  stable_hasher::Hash64,
};
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::{
  traits::{query::NoSolution, solve::Certainty},
  ty::{Ty, TyCtxt},
};
use rustc_span::{symbol::Symbol, Span};
use rustc_utils::source_map::range::{CharRange, ToSpan};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;

use self::intermediate::EvaluationResult;
use crate::serialize::{
  serialize_to_value,
  ty::{SymbolDef, TyDef},
};

// -----------------

index_vec::define_index_type! {
  pub struct ObligationIdx = u32;
}

index_vec::define_index_type! {
  pub struct ExprIdx = u32;
}

index_vec::define_index_type! {
  pub struct MethodLookupIdx = u32;
}

#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct MethodLookup {
  pub table: Vec<MethodStep>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct MethodStep {
  pub recvr_ty: ReceiverAdjStep,
  pub trait_predicates: Vec<ObligationIdx>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct ReceiverAdjStep {
  #[cfg_attr(feature = "ts-rs", ts(type = "any"))]
  ty: serde_json::Value,
}

impl ReceiverAdjStep {
  pub fn new<'tcx>(infcx: &InferCtxt<'tcx>, ty: Ty<'tcx>) -> Self {
    #[derive(Serialize)]
    struct Wrapper<'tcx>(#[serde(with = "TyDef")] Ty<'tcx>);
    let value =
      serialize_to_value(infcx, &Wrapper(ty)).expect("failed to serialize ty");
    ReceiverAdjStep { ty: value }
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct Expr {
  pub range: CharRange,
  pub obligations: HashSet<ObligationIdx>,
  pub kind: ExprKind,
}

#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ExprKind {
  Misc,
  CallableExpr,
  MethodReceiver,
  Call,
  CallArg,
  MethodCall {
    data: MethodLookupIdx,
    error_recvr: bool,
  },
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
  pub ambiguity_errors: HashSet<ExprIdx>,

  /// Concrete trait errors, this would be when the compiler
  /// can say for certainty that a specific trait bound was required
  /// but not satisfied.
  pub trait_errors: HashSet<ExprIdx>,

  pub obligations: IndexVec<ObligationIdx, Obligation>,

  pub exprs: IndexVec<ExprIdx, Expr>,

  pub method_lookups: IndexVec<MethodLookupIdx, MethodLookup>,
}

#[derive(Serialize, Clone, Debug)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase", tag = "type")]
pub struct Obligation {
  #[cfg_attr(feature = "ts-rs", ts(type = "any"))]
  /// Actual type: Predicate<'tcx>,
  pub predicate: serde_json::Value,
  pub hash: ObligationHash,
  pub range: CharRange,
  pub kind: ObligationKind,
  pub necessity: ObligationNecessity,
  #[serde(with = "intermediate::EvaluationResultDef")]
  pub result: intermediate::EvaluationResult,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
pub enum ObligationNecessity {
  No,
  ForProfessionals,
  OnError,
  Yes,
}

impl ObligationNecessity {
  pub fn is_necessary(&self, res: EvaluationResult) -> bool {
    use ObligationNecessity::*;
    matches!(
      (self, res),
      (Yes, _) // TODO: | (OnError, Err(..))
    )
  }
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

  use rustc_middle::traits::solve::MaybeCause;

  use super::*;

  pub type EvaluationResult = Result<Certainty, NoSolution>;

  pub struct EvaluationResultDef;
  impl EvaluationResultDef {
    pub fn serialize<S: serde::Serializer>(
      value: &EvaluationResult,
      s: S,
    ) -> Result<S::Ok, S::Error> {
      let string = match value {
        Ok(Certainty::Yes) => "yes",
        Ok(Certainty::Maybe(MaybeCause::Overflow)) => "maybe-overflow",
        Ok(Certainty::Maybe(MaybeCause::Ambiguity)) => "maybe-ambiguity",
        Err(..) => "no",
      };

      string.serialize(s)
    }
  }

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
