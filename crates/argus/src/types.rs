use std::{ops::Deref, str::FromStr};

use anyhow::Result;
use index_vec::IndexVec;
use indexmap::IndexSet;
use rustc_data_structures::stable_hasher::Hash64;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::{
  traits::{
    query::NoSolution,
    solve::{Certainty, MaybeCause},
  },
  ty::{self, Ty, TyCtxt, TypeckResults},
};
use rustc_span::{def_id::DefId, Span};
use rustc_utils::source_map::range::{CharRange, ToSpan};
use serde::{Deserialize, Serialize};
#[cfg(feature = "testing")]
use ts_rs::TS;

use self::intermediate::EvaluationResult;
use crate::{
  analysis::{FullObligationData, SynIdx, UODIdx},
  serialize::{
    safe::{PathDefNoArgs, TraitRefPrintOnlyTraitPathDef},
    serialize_to_value,
    ty::{
      ImplPolarityDef, RegionDef, Slice__ClauseDef, Slice__GenericArgDef,
      Slice__TyDef, TyDef,
    },
  },
};

// -----------------

crate::define_idx! { usize,
  ExprIdx,
  MethodLookupIdx,
  ObligationIdx
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct MethodLookup {
  pub candidates: ExtensionCandidates,
  pub table: Vec<MethodStep>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ExtensionCandidates {
  #[cfg_attr(feature = "testing", ts(type = "TraitRefPrintOnlyTraitPath[]"))]
  data: serde_json::Value,
}

impl ExtensionCandidates {
  pub fn new<'tcx>(
    infcx: &InferCtxt<'tcx>,
    traits: Vec<ty::TraitRef<'tcx>>,
  ) -> Self {
    let wrapped = traits
      .into_iter()
      .map(TraitRefPrintOnlyTraitPathDef)
      .collect::<Vec<_>>();
    let json = serialize_to_value(infcx, &wrapped)
      .expect("failed to serialied trait refs for method lookup");
    ExtensionCandidates { data: json }
  }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct MethodStep {
  pub recvr_ty: ReceiverAdjStep,
  pub trait_predicates: Vec<ObligationIdx>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ReceiverAdjStep {
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
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
  MethodReceiver,
  Call,
  CallArg,
  #[serde(rename_all = "camelCase")]
  MethodCall {
    data: MethodLookupIdx,
    error_recvr: bool,
  },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ObligationsInBody {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "testing", ts(type = "PathDefNoArgs | undefined"))]
  name: Option<serde_json::Value>,

  hash: BodyHash,

  /// Range of the represented body.
  pub range: CharRange,

  /// All ambiguous expression in the body. These *could* involve
  /// trait errors, so it's important that we can map the specific
  /// obligations to these locations. (That is, if they occur.)
  pub ambiguity_errors: IndexSet<ExprIdx>,

  /// Concrete trait errors, this would be when the compiler
  /// can say for certainty that a specific trait bound was required
  /// but not satisfied.
  pub trait_errors: Vec<(ExprIdx, Vec<ObligationHash>)>,

  #[cfg_attr(feature = "testing", ts(type = "Obligation[]"))]
  pub obligations: IndexVec<ObligationIdx, Obligation>,

  #[cfg_attr(feature = "testing", ts(type = "Expr[]"))]
  pub exprs: IndexVec<ExprIdx, Expr>,

  #[cfg_attr(feature = "testing", ts(type = "MethodLookup[]"))]
  pub method_lookups: IndexVec<MethodLookupIdx, MethodLookup>,
}

impl ObligationsInBody {
  pub fn new(
    id: Option<(&InferCtxt, DefId)>,
    range: CharRange,
    ambiguity_errors: IndexSet<ExprIdx>,
    trait_errors: Vec<(ExprIdx, Vec<ObligationHash>)>,
    obligations: IndexVec<ObligationIdx, Obligation>,
    exprs: IndexVec<ExprIdx, Expr>,
    method_lookups: IndexVec<MethodLookupIdx, MethodLookup>,
  ) -> Self {
    let json_name = id.and_then(|(infcx, id)| {
      serialize_to_value(infcx, &PathDefNoArgs(id)).ok()
    });
    ObligationsInBody {
      name: json_name,
      hash: BodyHash::new(),
      range,
      ambiguity_errors,
      trait_errors,
      obligations,
      exprs,
      method_lookups,
    }
  }
}

#[derive(Serialize)]
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
  pub obligation: serde_json::Value,
  pub hash: ObligationHash,
  pub range: CharRange,
  pub kind: ObligationKind,
  pub necessity: ObligationNecessity,
  #[serde(with = "intermediate::EvaluationResultDef")]
  #[cfg_attr(feature = "testing", ts(type = "EvaluationResult"))]
  pub result: intermediate::EvaluationResult,
  pub is_synthetic: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ImplHeader<'tcx> {
  #[serde(with = "Slice__GenericArgDef")]
  #[cfg_attr(feature = "testing", ts(type = "GenericArg[]"))]
  pub args: Vec<ty::GenericArg<'tcx>>,

  #[cfg_attr(feature = "testing", ts(type = "TraitRefPrintOnlyTraitPath"))]
  pub name: TraitRefPrintOnlyTraitPathDef<'tcx>,

  #[serde(with = "TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  pub self_ty: ty::Ty<'tcx>,

  pub predicates: GroupedClauses<'tcx>,

  #[serde(with = "Slice__TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty[]"))]
  pub tys_without_default_bounds: Vec<Ty<'tcx>>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct GroupedClauses<'tcx> {
  pub grouped: Vec<ClauseWithBounds<'tcx>>,
  #[serde(with = "Slice__ClauseDef")]
  #[cfg_attr(feature = "testing", ts(type = "Clause[]"))]
  pub other: Vec<ty::Clause<'tcx>>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ClauseWithBounds<'tcx> {
  #[serde(with = "TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  pub ty: ty::Ty<'tcx>,
  pub bounds: Vec<ClauseBound<'tcx>>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum ClauseBound<'tcx> {
  Trait(
    #[serde(with = "ImplPolarityDef")]
    #[cfg_attr(feature = "testing", ts(type = "ImplPolarity"))]
    ty::ImplPolarity,
    #[cfg_attr(feature = "testing", ts(type = "TraitRefPrintOnlyTraitPath"))]
    TraitRefPrintOnlyTraitPathDef<'tcx>,
  ),
  Region(
    #[serde(with = "RegionDef")]
    #[cfg_attr(feature = "testing", ts(type = "Region"))]
    ty::Region<'tcx>,
  ),
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
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
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum ObligationKind {
  Success,
  Ambiguous,
  Failure,
}

#[derive(Deserialize, Serialize, Copy, Clone, Debug, Hash, PartialEq, Eq)]
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

  use std::{
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
    ops::Deref,
  };

  use anyhow::Result;
  use rustc_hir::{hir_id::HirId, BodyId};

  use super::*;

  // The provenance about where an element came from,
  // or was "spawned from," in the HIR. This type is intermediate
  // but stored in the TLS, it shouldn't capture lifetimes but
  // can capture unstable hashes.
  pub(crate) struct Provenance<T: Sized> {
    // The expression from whence `it` came, the
    // referenced element is expected to be an
    // expression.
    pub hir_id: HirId,
    // Index into the full provenance data, this is stored for interesting obligations.
    pub full_data: Option<UODIdx>,
    pub synthetic_data: Option<SynIdx>,
    pub it: T,
  }

  impl<T: Sized> Provenance<T> {
    pub fn map<U: Sized>(&self, f: impl FnOnce(&T) -> U) -> Provenance<U> {
      Provenance {
        it: f(&self.it),
        hir_id: self.hir_id,
        full_data: self.full_data,
        synthetic_data: self.synthetic_data,
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
      self.it.hash(state)
    }
  }

  pub trait ForgetProvenance {
    type Target;
    fn forget(self) -> Self::Target;
  }

  impl<T: Sized> ForgetProvenance for Vec<Provenance<T>> {
    type Target = Vec<T>;
    fn forget(self) -> Self::Target {
      self.into_iter().map(|f| f.forget()).collect()
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
        Ok(Certainty::Maybe(MaybeCause::Overflow)) => "maybe-overflow",
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

  pub struct FullData<'tcx> {
    pub(crate) obligations: ObligationQueriesInBody<'tcx>,
    pub(crate) synthetic: SyntheticQueriesInBody<'tcx>,
  }

  impl<'tcx> FullData<'tcx> {
    pub(crate) fn iter<'me>(
      &'me self,
    ) -> impl Iterator<
      Item = (&PredicateObligation<'tcx>, &FullObligationData<'tcx>),
    > + 'me {
      self
        .synthetic
        .iter()
        .map(|sdata| {
          let fdata = &*self.obligations.get(sdata.full_data);
          let obligation = &sdata.obligation;
          (obligation, fdata)
        })
        .chain(
          self
            .obligations
            .iter()
            .map(|fdata| (&fdata.obligation, fdata)),
        )
    }
  }

  pub(crate) struct SyntheticData<'tcx> {
    // points to the used `InferCtxt`
    pub full_data: UODIdx,
    pub obligation: PredicateObligation<'tcx>,
  }

  pub(crate) struct SyntheticQueriesInBody<'tcx>(
    IndexVec<SynIdx, SyntheticData<'tcx>>,
  );

  impl<'tcx> SyntheticQueriesInBody<'tcx> {
    pub fn new() -> Self {
      SyntheticQueriesInBody(Default::default())
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &SyntheticData<'tcx>> {
      self.0.iter()
    }

    pub(crate) fn add(&mut self, data: SyntheticData<'tcx>) -> SynIdx {
      self.0.push(data)
    }
  }

  pub(crate) struct ObligationQueriesInBody<'tcx>(
    IndexVec<UODIdx, FullObligationData<'tcx>>,
  );

  impl<'tcx> ObligationQueriesInBody<'tcx> {
    pub(crate) fn new(v: IndexVec<UODIdx, FullObligationData<'tcx>>) -> Self {
      ObligationQueriesInBody(v)
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
}
