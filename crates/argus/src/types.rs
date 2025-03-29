use std::{collections::HashMap, hash::Hash, ops::Deref, str::FromStr};

use anyhow::Result;
use argus_ser::{self as ser, interner::TyIdx};
use index_vec::IndexVec;
use indexmap::IndexSet;
use rustc_hashes::Hash64;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::{
  traits::{
    query::NoSolution,
    solve::{Certainty, MaybeCause},
  },
  ty::{self, TyCtxt, TypeckResults},
};
use rustc_span::Span;
use rustc_utils::source_map::range::{CharRange, ToSpan};
use serde::{Deserialize, Serialize};
use serde_json as json;
#[cfg(feature = "testing")]
use ts_rs::TS;

pub use self::intermediate::{EvaluationResult, EvaluationResultDef};
use crate::{
  proof_tree::SerializedTree,
  tls::{self, FullObligationData, UODIdx},
};

ser::define_idx! { usize,
  ExprIdx,
  ObligationIdx
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct BodyBundle {
  pub filename: String,
  pub body: ObligationsInBody,
  pub trees: HashMap<ObligationHash, SerializedTree>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ExtensionCandidates {
  #[cfg_attr(feature = "testing", ts(type = "TraitRefPrintOnlyTraitPath[]"))]
  data: json::Value,
}

impl ExtensionCandidates {
  pub fn new<'tcx>(
    infcx: &InferCtxt<'tcx>,
    traits: Vec<ty::TraitRef<'tcx>>,
  ) -> Self {
    let wrapped = traits
      .into_iter()
      .map(ser::TraitRefPrintOnlyTraitPathDef)
      .collect::<Vec<_>>();
    let json = tls::unsafe_access_interner(|ty_interner| {
      ser::to_value_expect(infcx, ty_interner, &wrapped)
    });
    ExtensionCandidates { data: json }
  }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct Expr {
  pub range: CharRange,
  pub snippet: String,
  #[cfg_attr(feature = "testing", ts(type = "ObligationIdx[]"))]
  pub obligations: Vec<ObligationIdx>,
  pub kind: ExprKind,
  pub is_body: bool,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum ExprKind {
  Misc,
  CallableExpr,
  Call,
  CallArg,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct AmbiguityError {
  pub idx: ExprIdx,
  pub range: CharRange,
}

impl Hash for AmbiguityError {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.idx.hash(state);
  }
}

impl Eq for AmbiguityError {}
impl PartialEq for AmbiguityError {
  fn eq(&self, other: &Self) -> bool {
    self.idx.eq(&other.idx)
  }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct TraitError {
  pub idx: ExprIdx,
  pub range: CharRange,
  pub hashes: Vec<ObligationHash>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ObligationsInBody {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "testing", ts(type = "PathDefNoArgs | undefined"))]
  name: Option<json::Value>,

  hash: BodyHash,

  /// Range of the represented body.
  pub range: CharRange,

  pub is_tainted: bool,

  /// All ambiguous expression in the body. These *could* involve
  /// trait errors, so it's important that we can map the specific
  /// obligations to these locations. (That is, if they occur.)
  pub ambiguity_errors: IndexSet<AmbiguityError>,

  /// Concrete trait errors, this would be when the compiler
  /// can say for certainty that a specific trait bound was required
  /// but not satisfied.
  pub trait_errors: Vec<TraitError>,

  #[cfg_attr(feature = "testing", ts(type = "Obligation[]"))]
  pub obligations: IndexVec<ObligationIdx, Obligation>,

  #[cfg_attr(feature = "testing", ts(type = "Expr[]"))]
  pub exprs: IndexVec<ExprIdx, Expr>,

  #[cfg_attr(feature = "testing", ts(type = "TyVal[]"))]
  pub tys: IndexVec<TyIdx, json::Value>,
}

impl ObligationsInBody {
  pub fn new(
    name: Option<json::Value>,
    is_tainted: bool,
    range: CharRange,
    ambiguity_errors: IndexSet<AmbiguityError>,
    trait_errors: Vec<TraitError>,
    obligations: IndexVec<ObligationIdx, Obligation>,
    exprs: IndexVec<ExprIdx, Expr>,
  ) -> Self {
    let tys = tls::take_interned_tys();
    ObligationsInBody {
      name,
      hash: BodyHash::new(),
      range,
      is_tainted,
      ambiguity_errors,
      trait_errors,
      obligations,
      exprs,
      tys,
    }
  }
}

#[derive(Serialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct BodyHash(
  #[cfg_attr(feature = "testing", ts(type = "string"))] uuid::Uuid,
);

impl BodyHash {
  fn new() -> Self {
    Self(uuid::Uuid::new_v4())
  }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct Obligation {
  #[cfg_attr(feature = "testing", ts(type = "PredicateObligation"))]
  pub obligation: json::Value,
  pub hash: ObligationHash,
  pub range: CharRange,
  pub kind: ObligationKind,
  pub necessity: ObligationNecessity,
  #[serde(with = "EvaluationResultDef")]
  #[cfg_attr(feature = "testing", ts(type = "EvaluationResult"))]
  pub result: EvaluationResult,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum ObligationNecessity {
  No,
  OnError,
  Yes,
}

impl ObligationNecessity {
  pub fn is_necessary(&self, res: EvaluationResult) -> bool {
    matches!(
      (self, res),
      (ObligationNecessity::Yes, _) // TODO: | (OnError, Err(..))
    )
  }
}

#[derive(Serialize, Clone, Debug)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum ObligationKind {
  Success,
  Ambiguous,
  Failure,
}

#[derive(
  Deserialize,
  Serialize,
  Copy,
  Clone,
  Debug,
  Hash,
  PartialEq,
  Eq,
  PartialOrd,
  Ord,
)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ObligationHash(
  #[serde(with = "string")]
  #[cfg_attr(feature = "testing", ts(type = "string"))]
  u64,
);

#[derive(Debug, Copy, Clone)]
pub struct Target {
  pub hash: ObligationHash,
  pub span: Span,
}

pub trait ToTarget {
  fn to_target(self, tcx: TyCtxt) -> Result<Target>;
}

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

impl From<&ObligationHash> for ObligationHash {
  fn from(value: &Self) -> Self {
    *value
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
  use std::{
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
    mem::ManuallyDrop,
    ops::Deref,
  };

  use anyhow::Result;
  use rustc_hir::{hir_id::HirId, BodyId};

  use super::*;

  /// The provenance from where an element came, or was "spawned from,"
  /// in the HIR. This type is intermediate but stored in the TLS, it
  /// shouldn't capture lifetimes but can capture unstable hashes.
  pub(crate) struct Provenance<T: Sized> {
    /// The expression from whence `it` came, the referenced element
    /// is expected to be an expression.
    pub hir_id: HirId,

    /// Index into the full provenance data, this is stored for interesting obligations.
    pub full_data: Option<UODIdx>,

    /// The actual element.
    pub it: T,
  }

  impl<T: Sized> Provenance<T> {
    pub fn map<U: Sized>(&self, f: impl FnOnce(&T) -> U) -> Provenance<U> {
      Provenance {
        it: f(&self.it),
        hir_id: self.hir_id,
        full_data: self.full_data,
      }
    }
  }

  impl<T: Sized> Deref for Provenance<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
      &self.it
    }
  }

  impl<T: Sized> Provenance<T> {
    pub fn forget(self) -> T {
      self.it
    }
  }

  impl<T: Debug> Debug for Provenance<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
      write!(f, "Provenance<{:?}>", self.it)
    }
  }

  impl<T: PartialEq> PartialEq for Provenance<T> {
    fn eq(&self, other: &Self) -> bool {
      self.it == other.it
    }
  }

  impl<T: Eq> Eq for Provenance<T> {}

  impl<T: Hash> Hash for Provenance<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
      self.it.hash(state);
    }
  }

  #[allow(dead_code)]
  pub trait ForgetProvenance {
    type Target;
    fn forget(self) -> Self::Target;
  }

  impl<T: Sized> ForgetProvenance for Vec<Provenance<T>> {
    type Target = Vec<T>;
    fn forget(self) -> Self::Target {
      self.into_iter().map(Provenance::forget).collect()
    }
  }

  pub type EvaluationResult = Result<Certainty, NoSolution>;

  pub struct EvaluationResultDef;
  impl EvaluationResultDef {
    pub fn serialize<S: serde::Serializer>(
      value: &EvaluationResult,
      s: S,
    ) -> Result<S::Ok, S::Error> {
      let string = match value {
        Ok(Certainty::Yes) => "yes",
        Ok(Certainty::Maybe(MaybeCause::Overflow { .. })) => "maybe-overflow",
        Ok(Certainty::Maybe(MaybeCause::Ambiguity)) => "maybe-ambiguity",
        Err(..) => "no",
      };

      string.serialize(s)
    }
  }

  pub struct FulfillmentData<'a, 'tcx: 'a> {
    pub hash: ObligationHash,
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

  #[allow(dead_code)]
  pub struct ErrorAssemblyCtx<'a, 'tcx: 'a> {
    pub tcx: TyCtxt<'tcx>,
    pub body_id: BodyId,
    pub typeck_results: &'tcx TypeckResults<'tcx>,
    pub obligations: &'a Vec<Provenance<Obligation>>,
    pub obligation_data: &'a FullData<'tcx>,
  }

  #[derive(PartialEq)]
  pub struct FullData<'tcx>(IndexVec<UODIdx, FullObligationData<'tcx>>);

  impl<'tcx> FullData<'tcx> {
    pub(crate) fn new(v: IndexVec<UODIdx, FullObligationData<'tcx>>) -> Self {
      FullData(v)
    }

    pub(crate) fn get(&self, idx: UODIdx) -> &FullObligationData<'tcx> {
      &self.0[idx]
    }

    pub(crate) fn iter(
      &self,
    ) -> impl Iterator<Item = &FullObligationData<'tcx>> {
      self.0.iter()
    }
  }

  // FIXME: this is bad. Seriously. The structure is a workaround for
  // dropping `InferCtxt`s which causes a `delayed_span_bug` panic.
  // Definitely a result of what we're doing, but I'm not sure exactly
  // what our "bad behavior" is.
  pub struct Forgettable<T: Sized>(ManuallyDrop<T>);

  impl<T: Sized + PartialEq> PartialEq for Forgettable<T> {
    fn eq(&self, other: &Self) -> bool {
      self.0 == other.0
    }
  }

  impl<T: Sized> Forgettable<T> {
    pub fn new(value: T) -> Self {
      Self(ManuallyDrop::new(value))
    }
  }

  impl<T: Sized> Deref for Forgettable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
      &self.0
    }
  }

  impl<T: Sized> Drop for Forgettable<T> {
    fn drop(&mut self) {
      let inner = unsafe { ManuallyDrop::take(&mut self.0) };
      std::mem::forget(inner);
    }
  }
}
