use std::{ops::Deref, str::FromStr};

use anyhow::Result;
use index_vec::IndexVec;
use rustc_data_structures::{fx::FxHashSet as HashSet, stable_hasher::Hash64};
use rustc_hir::BodyId;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::{
  traits::{
    query::NoSolution,
    solve::{Certainty, MaybeCause},
  },
  ty::{Ty, TyCtxt, TypeckResults},
};
use rustc_span::{symbol::Symbol, Span};
use rustc_utils::source_map::range::{CharRange, ToSpan};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use self::intermediate::EvaluationResult;
use crate::{
  analysis::{FullObligationData, Provenance, SynIdx, UODIdx},
  serialize::{
    serialize_to_value,
    ty::{SymbolDef, TyDef},
  },
};

// -----------------

crate::define_idx! { usize,
  ExprIdx,
  MethodLookupIdx,
  ObligationIdx
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MethodLookup {
  pub table: Vec<MethodStep>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MethodStep {
  pub recvr_ty: ReceiverAdjStep,
  pub trait_predicates: Vec<ObligationIdx>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ReceiverAdjStep {
  #[ts(type = "any")] // type Ty
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

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Expr {
  pub range: CharRange,
  #[ts(type = "ObligationIdx[]")]
  pub obligations: HashSet<ObligationIdx>,
  pub kind: ExprKind,
}

#[derive(Serialize, TS)]
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

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ObligationsInBody {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(serialize_with = "serialize_option")]
  #[ts(type = "Symbol | undefined")]
  pub name: Option<Symbol>,

  /// Range of the represented body.
  pub range: CharRange,

  /// All ambiguous expression in the body. These *could* involve
  /// trait errors, so it's important that we can map the specific
  /// obligations to these locations. (That is, if they occur.)
  #[ts(type = "ExprIdx[]")]
  pub ambiguity_errors: HashSet<ExprIdx>,

  /// Concrete trait errors, this would be when the compiler
  /// can say for certainty that a specific trait bound was required
  /// but not satisfied.
  #[ts(type = "ExprIdx[]")]
  pub trait_errors: HashSet<ExprIdx>,

  pub obligations: IndexVec<ObligationIdx, Obligation>,

  pub exprs: IndexVec<ExprIdx, Expr>,

  pub method_lookups: IndexVec<MethodLookupIdx, MethodLookup>,
}

#[derive(Serialize, TS, Clone, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
pub struct Obligation {
  #[ts(type = "any")] // type: Predicate
  pub predicate: serde_json::Value,
  pub hash: ObligationHash,
  pub range: CharRange,
  pub kind: ObligationKind,
  pub necessity: ObligationNecessity,
  #[serde(with = "intermediate::EvaluationResultDef")]
  #[ts(type = "EvaluationResult")]
  pub result: intermediate::EvaluationResult,
  pub is_synthetic: bool,
}

#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
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

#[derive(Serialize, TS, Clone, Debug)]
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

#[derive(
  Deserialize, TS, Serialize, Copy, Clone, Debug, Hash, PartialEq, Eq,
)]
pub struct ObligationHash(
  #[serde(with = "string")]
  #[ts(type = "string")]
  u64,
);

#[derive(Debug)]
pub struct Target {
  pub hash: ObligationHash,
  pub span: Span,
  pub is_synthetic: bool,
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
      is_synthetic: false,
    })
  }
}

impl<U: Into<ObligationHash>, T: ToSpan> ToTarget for (U, T, bool) {
  fn to_target(self, tcx: TyCtxt) -> Result<Target> {
    self.1.to_span(tcx).map(|span| Target {
      hash: self.0.into(),
      span,
      is_synthetic: self.2,
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
    pub is_synthetic: bool,
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

  pub struct ErrorAssemblyCtx<'a, 'tcx: 'a> {
    pub tcx: TyCtxt<'tcx>,
    pub body_id: BodyId,
    pub typeck_results: &'tcx TypeckResults<'tcx>,
    pub obligations: &'a Vec<Provenance<Obligation>>,
    pub obligation_data: &'a ObligationQueriesInBody<'tcx>,
  }

  pub(crate) struct SyntheticData<'tcx> {
    // points to the used `InferCtxt`
    pub full_data: UODIdx,
    pub obligation: PredicateObligation<'tcx>,
    pub result: EvaluationResult,
  }

  pub(crate) struct SyntheticQueriesInBody<'tcx>(
    IndexVec<SynIdx, SyntheticData<'tcx>>,
  );

  impl<'tcx> SyntheticQueriesInBody<'tcx> {
    pub fn new() -> Self {
      SyntheticQueriesInBody(Default::default())
    }

    pub fn into_iter(self) -> impl Iterator<Item = SyntheticData<'tcx>> {
      self.0.into_iter()
    }

    pub fn add(&mut self, data: SyntheticData<'tcx>) -> SynIdx {
      self.0.push(data)
    }
  }

  pub(crate) struct ObligationQueriesInBody<'tcx>(
    IndexVec<UODIdx, FullObligationData<'tcx>>,
  );

  impl<'tcx> ObligationQueriesInBody<'tcx> {
    pub fn new(v: IndexVec<UODIdx, FullObligationData<'tcx>>) -> Self {
      ObligationQueriesInBody(v)
    }

    pub fn get(&self, idx: UODIdx) -> &FullObligationData<'tcx> {
      &self.0[idx]
    }
  }
}
