//! Remote serde::Serialize derives for Rustc types
//!
//! WARNING, these definitions were done hastily, and definitely
//! need a little "fixing up." It will be done at some point.
//! In the meantime, consume at your own risk.
#![allow(
    non_camel_case_types,
    non_snake_case,
    suspicious_double_ref_op,
    dead_code
)]

mod r#const;
mod path;
mod term;
mod my_ty;

use std::num::*;

use rustc_type_ir as ir;
use rustc_infer::infer::{InferCtxt, type_variable::TypeVariableOriginKind};
use rustc_middle::{ty::{self, *, abstract_const::CastKind}, mir::{BinOp, UnOp}};
use rustc_hir::def_id::{DefId, DefIndex, CrateNum};
use rustc_span::symbol::Symbol;
use rustc_target::spec::abi::Abi;
use rustc_hir::Unsafety;
use rustc_trait_selection::traits::solve::Goal;

use serde::{Serialize, ser::SerializeSeq};

use r#const::*;
use path::*;
use term::*;
use my_ty::*;

/// Entry function to serialize anything from rustc.
pub fn serialize_to_value<'tcx, T: Serialize + 'tcx>(
    value: &T, infcx: &InferCtxt<'tcx>
) -> Result<serde_json::Value, serde_json::Error> {
    in_dynamic_ctx(infcx, || serde_json::to_value(&value))
}

// NOTE: setting the dynamic TCX should *only* happen
// before calling the serialize function, it must guarantee
// that the 'tcx lifetime is the same as that of the serialized item.
fluid_let::fluid_let!{static INFCX: &'static InferCtxt<'static>}

fn in_dynamic_ctx<'tcx, T>(infcx: &InferCtxt<'tcx>, f: impl FnOnce() -> T) -> T {
    let infcx: &'static InferCtxt<'static> = unsafe { std::mem::transmute(infcx) };
    INFCX.set(infcx, f)
}

fn get_dynamic_ctx<'a, 'tcx: 'a>() -> &'a InferCtxt<'tcx> {
    let infcx: &'static InferCtxt<'static> = INFCX.copied().unwrap();
    unsafe { std::mem::transmute::<
            &'static InferCtxt<'static>,
            &'a InferCtxt<'tcx>
        >(infcx) }
}

// TODO: we could also generate the functions
macro_rules! serialize_custom_seq {
    ($wrap:ident, $serializer:expr, $value:expr) => {{
        let mut seq = $serializer.serialize_seq(Some($value.len()))?;
        for e in $value.iter() {
            seq.serialize_element(&$wrap(e))?;
        }
        seq.end()
    }}
}

pub(crate) use serialize_custom_seq;

// -----------------------------------------------
// Serializing types, you most likely (definitely)
// don't want to look in this module.

pub fn goal__predicate_def<'tcx, S>(value: &Goal<'tcx, Predicate<'tcx>>, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer
{
    Goal__PredicateDef::from(value).serialize(s)
}

#[derive(Serialize)]
pub struct Goal__PredicateDef<'tcx> {
    #[serde(with = "PredicateDef")]
    pub predicate: Predicate<'tcx>,
    #[serde(with = "ParamEnvDef")]
    pub param_env: ParamEnv<'tcx>,
}

impl<'tcx> From<&Goal<'tcx, Predicate<'tcx>>> for Goal__PredicateDef<'tcx> {
    fn from(value: &Goal<'tcx, Predicate<'tcx>>) -> Self {
        Goal__PredicateDef { 
            predicate: value.predicate.clone(), 
            param_env: value.param_env 
        }
    }
}

pub struct ParamEnvDef;
impl ParamEnvDef {
    pub fn serialize<'tcx, S>(value: &ParamEnv<'tcx>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'tcx>(#[serde(with = "ClauseDef")] Clause<'tcx>);
        serialize_custom_seq! { Wrapper, s, value.caller_bounds() }
    }
}

pub struct PredicateDef;
impl PredicateDef {
    pub fn serialize<'tcx, S>(value: &Predicate<'tcx>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        log::debug!("Serializing predicate {:#?}", value);
        Binder__PredicateKind::from(&value.kind()).serialize(s)
    }
}

#[derive(Serialize)]
pub struct Binder__ClauseKindDef;
impl Binder__ClauseKindDef {
    pub fn serialize<'tcx, S>(value: &Binder<'tcx, ClauseKind<'tcx>>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'tcx> {
            #[serde(serialize_with = "vec__bound_variable_kind")]
            pub bound_vars: Vec<BoundVariableKind>,
            #[serde(serialize_with = "clause_kind__ty_ctxt")]
            pub value: ir::ClauseKind<TyCtxt<'tcx>>,
        }

        Wrapper {
            bound_vars: value.bound_vars().to_vec(),
            value: value.skip_binder(),
        }.serialize(s)
    }
}



#[derive(Serialize)]
pub struct Binder__PredicateKind<'tcx> {
    #[serde(serialize_with = "vec__bound_variable_kind")]
    pub bound_vars: Vec<BoundVariableKind>,
    #[serde(with = "PredicateKindDef")]
    pub value: ir::PredicateKind<TyCtxt<'tcx>>,
}

impl<'tcx> From<&Binder<'tcx, ir::PredicateKind<TyCtxt<'tcx>>>> for Binder__PredicateKind<'tcx> {
    fn from(value: &Binder<'tcx, ir::PredicateKind<TyCtxt<'tcx>>>) -> Self {
        Binder__PredicateKind {
            bound_vars: value.bound_vars().to_vec(),
            value: value.skip_binder().clone(),
        }
    }
}

fn vec__bound_variable_kind<S>(value: &Vec<BoundVariableKind>, s: S) -> Result<S::Ok, S::Error>
    where
    S: serde::Serializer,
{
    #[derive(Serialize)]
    struct Wrapper<'a>(#[serde(with = "BoundVariableKindDef")] &'a BoundVariableKind);
    serialize_custom_seq! { Wrapper, s, value }
}

pub struct BoundExistentialPredicatesDef;
impl BoundExistentialPredicatesDef {
    pub fn serialize<'tcx, S>(value: &List<Binder<'tcx, ExistentialPredicate<'tcx>>>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'tcx>(#[serde(serialize_with = "binder__existential_predicate")] Binder<'tcx, ExistentialPredicate<'tcx>>);
        serialize_custom_seq! { Wrapper, s, value }
    }
}

fn binder__existential_predicate<'tcx, S>(value: &Binder<'tcx, ExistentialPredicate<'tcx>>, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    Binder__ExistentialPredicate::from(value).serialize(s)
}

#[derive(Serialize)]
pub struct Binder__ExistentialPredicate<'tcx> {
    #[serde(serialize_with = "vec__bound_variable_kind")]
    pub bound_vars: Vec<BoundVariableKind>,
    #[serde(with = "ExistentialPredicateDef")]
    pub value: ExistentialPredicate<'tcx>,
}

impl<'tcx> From<&Binder<'tcx, ExistentialPredicate<'tcx>>> for Binder__ExistentialPredicate<'tcx> {
    fn from(value: &Binder<'tcx, ExistentialPredicate<'tcx>>) -> Self {
        Binder__ExistentialPredicate {
            bound_vars: value.bound_vars().to_vec(),
            value: value.skip_binder().clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(remote = "ExistentialPredicate")]
pub enum ExistentialPredicateDef<'tcx> {
    Trait(#[serde(with = "ExistentialTraitRefDef")] ExistentialTraitRef<'tcx>),
    Projection(#[serde(with = "ExistentialProjectionDef")] ExistentialProjection<'tcx>),
    AutoTrait(#[serde(with = "DefIdDef")] DefId),
}

#[derive(Serialize)]
#[serde(remote = "ExistentialTraitRef")]
pub struct ExistentialTraitRefDef<'tcx> {
    #[serde(with = "DefIdDef")]
    pub def_id: DefId,
    #[serde(with = "GenericArgsDef")]
    pub args: GenericArgsRef<'tcx>,
}

#[derive(Serialize)]
#[serde(remote = "PredicateKind")]
pub enum PredicateKindDef<'tcx> {
    Clause(#[serde(serialize_with = "clause_kind__ty_ctxt")] ir::ClauseKind<TyCtxt<'tcx>>),
    ObjectSafe(#[serde(with = "DefIdDef")] DefId),
    Subtype(#[serde(with = "SubtypePredicateDef")] SubtypePredicate<'tcx>),
    Coerce(#[serde(with = "CoercePredicateDef")] CoercePredicate<'tcx>),
    ConstEquate(#[serde(with = "ConstDef")] Const<'tcx>, #[serde(with = "ConstDef")] Const<'tcx>),
    Ambiguous,
    AliasRelate(#[serde(with = "TermDef")] Term<'tcx>, #[serde(with = "TermDef")] Term<'tcx>, #[serde(with = "AliasRelationDirectionDef")] AliasRelationDirection),
    ClosureKind(#[serde(with = "DefIdDef")] DefId, #[serde(with = "GenericArgsDef")] GenericArgsRef<'tcx>, #[serde(with = "ClosureKindDef")] ClosureKind),
}

#[derive(Serialize)]
#[serde(remote = "ClosureKind")]
pub enum ClosureKindDef {
    Fn,
    FnMut,
    FnOnce,
}

pub struct ClauseDef;
impl ClauseDef {
    fn serialize<'tcx, S>(value: &Clause<'_>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Binder__ClauseKindDef::serialize(&value.kind(), s)
    }
}

fn clause_kind__ty_ctxt<'tcx, S>(value: &ir::ClauseKind<TyCtxt<'tcx>>, s: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
    ClauseKind__TyCtxt::from(value).serialize(s)
}

#[derive(Serialize)]
pub enum ClauseKind__TyCtxt<'tcx> {
    Trait(#[serde(with = "TraitPredicateDef")] <TyCtxt<'tcx> as Interner>::TraitPredicate),
    RegionOutlives(#[serde(with = "RegionOutlivesPredicateDef")] <TyCtxt<'tcx> as Interner>::RegionOutlivesPredicate),
    TypeOutlives(#[serde(with = "TypeOutlivesPredicateDef")] <TyCtxt<'tcx> as Interner>::TypeOutlivesPredicate),
    Projection(#[serde(with = "ProjectionPredicateDef")] <TyCtxt<'tcx> as Interner>::ProjectionPredicate),
    ConstArgHasType(#[serde(with = "ConstDef")] <TyCtxt<'tcx> as Interner>::Const, #[serde(with = "TyDef")] <TyCtxt<'tcx> as Interner>::Ty),
    WellFormed(#[serde(with = "GenericArgDef")] <TyCtxt<'tcx> as Interner>::GenericArg),
    ConstEvaluatable(#[serde(with = "ConstDef")] <TyCtxt<'tcx> as Interner>::Const),
}

impl<'tcx> From<&ir::ClauseKind<TyCtxt<'tcx>>> for ClauseKind__TyCtxt<'tcx> {
    fn from(value: &ir::ClauseKind<TyCtxt<'tcx>>) -> Self {
        match value {
            ClauseKind::Trait(v) =>               ClauseKind__TyCtxt::Trait(v.clone()),
            ClauseKind::RegionOutlives(v) =>      ClauseKind__TyCtxt::RegionOutlives(v.clone()),
            ClauseKind::TypeOutlives(v) =>        ClauseKind__TyCtxt::TypeOutlives(v.clone()),
            ClauseKind::Projection(v) =>          ClauseKind__TyCtxt::Projection(v.clone()),
            ClauseKind::ConstArgHasType(v1, v2) =>ClauseKind__TyCtxt::ConstArgHasType(v1.clone(), v2.clone()),
            ClauseKind::WellFormed(v) =>          ClauseKind__TyCtxt::WellFormed(v.clone()),
            ClauseKind::ConstEvaluatable(v) =>    ClauseKind__TyCtxt::ConstEvaluatable(v.clone()),
        }
    }
}

#[derive(Serialize)]
#[serde(remote = "SubtypePredicate")]
pub struct SubtypePredicateDef<'tcx> {
    pub a_is_expected: bool,
    #[serde(with = "TyDef")]
    pub a: Ty<'tcx>,
    #[serde(with = "TyDef")]
    pub b: Ty<'tcx>,
}

#[derive(Serialize)]
#[serde(remote = "TraitPredicate")]
pub struct TraitPredicateDef<'tcx> {
    #[serde(with = "TraitRefDef")]
    pub trait_ref: TraitRef<'tcx>,
    #[serde(with = "ImplPolarityDef")]
    pub polarity: ImplPolarity,
}

#[derive(Serialize)]
pub struct TraitRefPrintOnlyTraitPathDefWrapper<'tcx>(
    #[serde(with = "TraitRefPrintOnlyTraitPathDef")] 
    pub TraitRef<'tcx>
);

pub struct TraitRefDef;
impl TraitRefDef {
    pub fn serialize<'tcx, S>(value: &TraitRef<'tcx>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'a, 'tcx: 'a> {
            #[serde(with = "TyDef")]
            pub self_ty: Ty<'tcx>,
            #[serde(with = "TraitRefPrintOnlyTraitPathDef")]
            // NOTE: we can't use the actual TraitRefPrintOnlyTraitPath because
            // the newtype wrapper makes the .0 field private. However, all it
            // does is wrap a TraitRef to print differently which we
            // do in the TraitRefPrintOnlyTraitPathDef::serialize function.
            pub trait_path: &'a TraitRef<'tcx>,
        }

        Wrapper {
            self_ty: value.self_ty(),
            trait_path: value,
        }.serialize(s)
    }
}

pub struct TraitRefPrintOnlyTraitPathDef;
impl TraitRefPrintOnlyTraitPathDef {
    pub fn serialize<'tcx, S>(value: &TraitRef<'tcx>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        log::debug!("Serializing trait ref {:#?}", value);
        path::PathDefWithArgs::new(value.def_id, value.args).serialize(s)
    }
}

#[derive(Serialize)]
#[serde(remote = "ImplPolarity")]
pub enum ImplPolarityDef {
    Positive,
    Negative,
    Reservation,
}

pub struct RegionOutlivesPredicateDef;
impl RegionOutlivesPredicateDef {
    pub fn serialize<'tcx, S>(value: &OutlivesPredicate<Region<'tcx>, Region<'tcx>>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        OutlivesPredicate__Region__Region::from(value).serialize(s)
    }
}

#[derive(Serialize)]
pub struct OutlivesPredicate__Region__Region<'tcx> {
    #[serde(with = "RegionDef")] 
    pub a: Region<'tcx>,
    #[serde(with = "RegionDef")] 
    pub b: Region<'tcx>,
}

impl<'tcx> From<&OutlivesPredicate<Region<'tcx>, Region<'tcx>>> for OutlivesPredicate__Region__Region<'tcx> {
    fn from(value: &OutlivesPredicate<Region<'tcx>, Region<'tcx>>) -> Self {
        OutlivesPredicate__Region__Region {
            a: value.0.clone(), 
            b: value.1.clone()
        }
    }
}

pub struct TypeOutlivesPredicateDef;
impl TypeOutlivesPredicateDef {
    pub fn serialize<'tcx, S>(value: &OutlivesPredicate<Ty<'tcx>, Region<'tcx>>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        OutlivesPredicate__Ty__Region::from(value).serialize(s)
    }
}

#[derive(Serialize)]
pub struct OutlivesPredicate__Ty__Region<'tcx> {
    #[serde(with = "TyDef")]
    pub a: Ty<'tcx>,
    #[serde(with = "RegionDef")]
    pub b: Region<'tcx>,
}

impl<'tcx> From<&OutlivesPredicate<Ty<'tcx>, Region<'tcx>>> for OutlivesPredicate__Ty__Region<'tcx> {
    fn from(value: &OutlivesPredicate<Ty<'tcx>, Region<'tcx>>) -> Self {
        OutlivesPredicate__Ty__Region {
            a: value.0.clone(),
            b: value.1.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(remote = "ProjectionPredicate")]
pub struct ProjectionPredicateDef<'tcx> {
    #[serde(with = "AliasTyDef")]
    pub projection_ty: AliasTy<'tcx>,
    #[serde(with = "TermDef")]
    pub term: Term<'tcx>,
}

#[derive(Serialize)]
#[serde(remote = "UniverseIndex")]
pub struct UniverseIndexDef {
    #[serde(skip)]
    pub(crate) private: u32,
}

pub struct DefIdDef;
impl DefIdDef {
    pub fn serialize<'tcx, S>(value: &DefId, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        log::warn!("Printing raw DefId {value:?} without generic args. Did you use to mean `path::PathDefWithArgs`?");
        path::path_def_no_args(*value, s)
    }
}

#[derive(Serialize)]
#[serde(remote = "CoercePredicate")]
pub struct CoercePredicateDef<'tcx> {
    #[serde(with = "TyDef")]
    pub a: Ty<'tcx>,
    #[serde(with = "TyDef")]
    pub b: Ty<'tcx>,
}

#[derive(Serialize)]
#[serde(remote = "InferConst")]
pub enum InferConstDef {
    Var(#[serde(with = "ConstVidDef")] ConstVid),
    EffectVar(#[serde(with = "EffectVidDef")] EffectVid),
    Fresh(u32),
}

#[derive(Serialize)]
#[serde(remote = "ConstVid")]
pub struct ConstVidDef {
    #[serde(skip)]
    private: u32,
}

#[derive(Serialize)]
#[serde(remote = "EffectVid")]
pub struct EffectVidDef {
    #[serde(skip)]
    private: u32,
}

#[derive(Serialize)]
#[serde(remote = "EarlyParamRegion")]
pub struct EarlyParamRegionDef {
    #[serde(with = "DefIdDef")]
    pub def_id: DefId,
    pub index: u32,
    #[serde(with = "SymbolDef")]
    pub name: Symbol,
}

pub struct InferRegionDef;
impl InferRegionDef {
    pub fn serialize<'tcx, S>(value: &RegionVid, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        RegionVidDef::serialize(value, s)
    }
}

#[derive(Serialize)]
#[serde(remote = "RegionVid")]
pub struct RegionVidDef {
    #[serde(skip)]
    private: u32,
}

pub struct PlaceholderRegionDef;
impl PlaceholderRegionDef {
    pub fn serialize<'tcx, S>(value: &Placeholder<BoundRegion>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        Placeholder__BoundRegion::from(value).serialize(s)
    }
}

fn slice__val_tree<'tcx, S>(value: &[ValTree<'tcx>], s: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer
{
    #[derive(Serialize)]
    struct Wrapper<'a, 'tcx>(#[serde(with = "ValTreeDef")] &'a ValTree<'tcx>);
    serialize_custom_seq! { Wrapper, s, value }
}

// FIXME:
#[derive(Serialize)]
#[serde(remote = "ScalarInt")]
pub struct ScalarIntDef {
    #[serde(skip)]
    data: u128,
    #[serde(skip)]
    size: NonZeroU8,
}

#[derive(Serialize)]
#[serde(remote = "ParamConst")]
pub struct ParamConstDef {
    pub index: u32,
    #[serde(with = "SymbolDef")]
    pub name: Symbol,
}

#[derive(Serialize)]
#[serde(remote = "ParamTy")]
pub struct ParamTyDef {
    pub index: u32,
    #[serde(with = "SymbolDef")]
    pub name: Symbol,
}

pub struct SymbolDef;
impl SymbolDef {
    pub fn serialize<'tcx, S>(value: &Symbol, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        value.as_str().serialize(s)
    }
}

#[derive(Serialize)]
#[serde(remote = "AliasRelationDirection")]
pub enum AliasRelationDirectionDef {
    Equate,
    Subtype,
}
