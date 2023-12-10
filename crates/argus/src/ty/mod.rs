//! Remote serde::Serialize derives for Rustc types
#![allow(
    non_camel_case_types,
    non_snake_case,
    suspicious_double_ref_op // FIXME: get rid of this eventually
)]

use std::num::*;

use rustc_type_ir as ir;
use rustc_middle::{ty::{*, abstract_const::CastKind}, mir::{BinOp, UnOp}};
use rustc_hir::def_id::{DefId, DefIndex, CrateNum};
use rustc_span::symbol::Symbol;
use rustc_target::spec::abi::Abi;
use rustc_hir::Unsafety;

use serde::{Serialize, ser::SerializeSeq};

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

pub struct PredicateDef;
impl PredicateDef {
    pub fn serialize<'tcx, S>(value: &Predicate<'tcx>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Binder__PredicateKind::from(&value.kind()).serialize(s)
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
#[serde(remote = "ExistentialProjection")]
pub struct ExistentialProjectionDef<'tcx> {
    #[serde(with = "DefIdDef")]
    pub def_id: DefId,
    #[serde(with = "GenericArgsDef")]
    pub args: GenericArgsRef<'tcx>,
    #[serde(with = "TermDef")]
    pub term: Term<'tcx>,
}

#[derive(Serialize)]
#[serde(remote = "DynKind")]
pub enum DynKindDef {
    Dyn,
    DynStar,
}

#[derive(Serialize)]
#[serde(remote = "Movability")]
pub enum MovabilityDef {
    Static,
    Movable,
}

#[derive(Serialize)]
#[serde(remote = "AliasKind")]
pub enum AliasKindDef {
    Projection,
    Inherent,
    Opaque,
    Weak,
}

pub struct TysDef;
impl TysDef {
    pub fn serialize<'tcx, S>(value: &List<Ty<'tcx>>, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'tcx>(#[serde(with = "TyDef")] Ty<'tcx>);
        serialize_custom_seq! { Wrapper, s, value }
    }
}

#[derive(Serialize)]
#[serde(remote = "AliasTy")]
pub struct AliasTyDef<'tcx> {
    #[serde(with = "GenericArgsDef")]
    pub args: GenericArgsRef<'tcx>,
    #[serde(with = "DefIdDef")]
    pub def_id: DefId,
    #[serde(skip)]
    _use_alias_ty_new_instead: (),
}

#[derive(Serialize)]
#[serde(remote = "BoundTy")]
pub struct BoundTyDef {
    #[serde(with = "BoundVarDef")]
    pub var: BoundVar,
    #[serde(with = "BoundTyKindDef")]
    pub kind: BoundTyKind,
}

#[derive(Serialize)]
#[serde(remote = "BoundVariableKind")]
pub enum BoundVariableKindDef {
    Ty(#[serde(with = "BoundTyKindDef")] BoundTyKind),
    Region(#[serde(with = "BoundRegionKindDef")] BoundRegionKind),
    Const,
}

#[derive(Serialize)]
#[serde(remote = "BoundRegionKind")]
pub enum BoundRegionKindDef {
    BrAnon,
    BrNamed(#[serde(with = "DefIdDef")] DefId, #[serde(with = "SymbolDef")] Symbol),
    BrEnv,
}

#[derive(Serialize)]
#[serde(remote = "BoundTyKind")]
pub enum BoundTyKindDef {
    Anon,
    Param(#[serde(with = "DefIdDef")] DefId, #[serde(with = "SymbolDef")] Symbol),
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

pub struct TyDef;
impl TyDef {
    pub fn serialize<'tcx, S>(value: &Ty<'tcx>, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            TyKindDef::serialize(value.kind(), s)
        }
}

pub struct TyKindDef;
impl TyKindDef {
    pub fn serialize<'tcx, S>(value: &TyKind<'tcx>, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            TyKind__TyCtxt::from(value).serialize(s)
        }
}

#[derive(Serialize)]
pub enum TyKind__TyCtxt<'tcx> {
    Bool,
    Char,
    Int(#[serde(with = "IntTyDef")] IntTy),
    Uint(#[serde(with = "UintTyDef")] UintTy),
    Float(#[serde(with = "FloatTyDef")] FloatTy),
    Adt(#[serde(with = "AdtDefDef")] <TyCtxt<'tcx> as Interner>::AdtDef, #[serde(with = "GenericArgsDef")] <TyCtxt<'tcx> as Interner>::GenericArgs),
    Foreign(#[serde(with = "DefIdDef")] <TyCtxt<'tcx> as Interner>::DefId),
    Str,
    Array(#[serde(with = "TyDef")] <TyCtxt<'tcx> as Interner>::Ty, #[serde(with = "ConstDef")] <TyCtxt<'tcx> as Interner>::Const),
    Slice(#[serde(with = "TyDef")] <TyCtxt<'tcx> as Interner>::Ty),
    RawPtr(#[serde(with = "TypeAndMutDef")] <TyCtxt<'tcx> as Interner>::TypeAndMut),
    Ref(#[serde(with = "RegionDef")] <TyCtxt<'tcx> as Interner>::Region, #[serde(with = "TyDef")] <TyCtxt<'tcx> as Interner>::Ty, #[serde(with = "MutabilityDef")] Mutability),
    FnDef(#[serde(with = "DefIdDef")] <TyCtxt<'tcx> as Interner>::DefId, #[serde(with = "GenericArgsDef")] <TyCtxt<'tcx> as Interner>::GenericArgs),
    FnPtr(#[serde(with = "PolyFnSigDef")] <TyCtxt<'tcx> as Interner>::PolyFnSig),
    Dynamic(#[serde(with = "BoundExistentialPredicatesDef")] <TyCtxt<'tcx> as Interner>::BoundExistentialPredicates, #[serde(with = "RegionDef")] <TyCtxt<'tcx> as Interner>::Region, #[serde(with = "DynKindDef")] DynKind),
    Closure(#[serde(with = "DefIdDef")] <TyCtxt<'tcx> as Interner>::DefId, #[serde(with = "GenericArgsDef")] <TyCtxt<'tcx> as Interner>::GenericArgs),
    Coroutine(#[serde(with = "DefIdDef")] <TyCtxt<'tcx> as Interner>::DefId, #[serde(with = "GenericArgsDef")] <TyCtxt<'tcx> as Interner>::GenericArgs, #[serde(with = "MovabilityDef")] Movability),
    CoroutineWitness(#[serde(with = "DefIdDef")] <TyCtxt<'tcx> as Interner>::DefId, #[serde(with = "GenericArgsDef")] <TyCtxt<'tcx> as Interner>::GenericArgs),
    Never,
    Tuple(#[serde(with = "TysDef")] <TyCtxt<'tcx> as Interner>::Tys),
    Alias(#[serde(with = "AliasKindDef")] AliasKind, #[serde(with = "AliasTyDef")] <TyCtxt<'tcx> as Interner>::AliasTy),
    Param(#[serde(with = "ParamTyDef")] <TyCtxt<'tcx> as Interner>::ParamTy),
    Bound(#[serde(skip)] DebruijnIndex, #[serde(with = "BoundTyDef")] <TyCtxt<'tcx> as Interner>::BoundTy),
    Placeholder(#[serde(with = "PlaceholderTyDef")] <TyCtxt<'tcx> as Interner>::PlaceholderTy),
    Infer(#[serde(with = "InferTyDef")] InferTy),
    Error(#[serde(skip)] <TyCtxt<'tcx> as Interner>::ErrorGuaranteed),
}

impl<'tcx> From<&ir::TyKind<TyCtxt<'tcx>>> for TyKind__TyCtxt<'tcx> {
    fn from(value: &ir::TyKind<TyCtxt<'tcx>>) -> Self {
        match value {
           TyKind::Bool =>                               TyKind__TyCtxt::Bool,
           TyKind::Char =>                               TyKind__TyCtxt::Char,
           TyKind::Int(v) =>                             TyKind__TyCtxt::Int(v.clone()),
           TyKind::Uint(v) =>                            TyKind__TyCtxt::Uint(v.clone()),
           TyKind::Float(v) =>                           TyKind__TyCtxt::Float(v.clone()),
           TyKind::Adt(v1,  v2) =>                       TyKind__TyCtxt::Adt(v1.clone(), v2.clone()),
           TyKind::Foreign(v) =>                         TyKind__TyCtxt::Foreign(v.clone()),
           TyKind::Str =>                                TyKind__TyCtxt::Str,
           TyKind::Array(v1,  v2) =>                     TyKind__TyCtxt::Array(v1.clone(), v2.clone()),
           TyKind::Slice(v) =>                           TyKind__TyCtxt::Slice(v.clone()),
           TyKind::RawPtr(v) =>                          TyKind__TyCtxt::RawPtr(v.clone()),
           TyKind::Ref(v1, v2, v3) =>                    TyKind__TyCtxt::Ref(v1.clone(), v2.clone(), v3.clone()),
           TyKind::FnDef(v1,  v2) =>                     TyKind__TyCtxt::FnDef(v1.clone(), v2.clone()),
           TyKind::FnPtr(v) =>                           TyKind__TyCtxt::FnPtr(v.clone()),
           TyKind::Dynamic(v1,  v2, v3) =>               TyKind__TyCtxt::Dynamic(v1.clone(), v2.clone(), v3.clone()),
           TyKind::Closure(v1,  v2) =>                   TyKind__TyCtxt::Closure(v1.clone(), v2.clone()),
           TyKind::Coroutine(v1,  v2, v3) =>             TyKind__TyCtxt::Coroutine(v1.clone(), v2.clone(), v3.clone()),
           TyKind::CoroutineWitness(v1,  v2) =>          TyKind__TyCtxt::CoroutineWitness(v1.clone(), v2.clone()),
           TyKind::Never =>                              TyKind__TyCtxt::Never,
           TyKind::Tuple(v) =>                           TyKind__TyCtxt::Tuple(v.clone()),
           TyKind::Alias(v1,  v2) =>                     TyKind__TyCtxt::Alias(v1.clone(), v2.clone()),
           TyKind::Param(v) =>                           TyKind__TyCtxt::Param(v.clone()),
           TyKind::Bound(v1,  v2) =>                     TyKind__TyCtxt::Bound(v1.clone(), v2.clone()),
           TyKind::Placeholder(v) =>                     TyKind__TyCtxt::Placeholder(v.clone()),
           TyKind::Infer(v) =>                           TyKind__TyCtxt::Infer(v.clone()),
           TyKind::Error(v) =>                           TyKind__TyCtxt::Error(v.clone()),
        }
    }
}

pub struct PlaceholderTyDef;
impl PlaceholderTyDef {
    pub fn serialize<'tcx, S>(value: &Placeholder<BoundTy>, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
    {
        Placeholder__BoundTy::from(value).serialize(s)
    }
}

#[derive(Serialize)]
pub struct Placeholder__BoundTy {
    #[serde(with = "UniverseIndexDef")]
    pub universe: UniverseIndex,
    #[serde(with = "BoundTyDef")]
    pub bound: BoundTy,
}

impl From<&Placeholder<BoundTy>> for Placeholder__BoundTy {
    fn from(value: &Placeholder<BoundTy> ) -> Self {
        Placeholder__BoundTy {
            universe: value.universe.clone(),
            bound: value.bound.clone(),
        }
    }
}


#[derive(Serialize)]
pub struct Placeholder__BoundRegion {
    #[serde(with = "UniverseIndexDef")]
    pub universe: UniverseIndex,
    #[serde(with = "BoundRegionDef")]
    pub bound: BoundRegion,
}

impl From<&Placeholder<BoundRegion>> for Placeholder__BoundRegion {
    fn from(value: &Placeholder<BoundRegion> ) -> Self {
        Placeholder__BoundRegion {
            universe: value.universe.clone(),
            bound: value.bound.clone(),
        }
    }
}

pub struct PolyFnSigDef;
impl PolyFnSigDef {
    pub fn serialize<'tcx, S>(value: &Binder<'tcx, FnSig<'tcx>>, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
    {
        Binder__FnSig::from(value).serialize(s)
    }
}

#[derive(Serialize)]
pub struct Binder__FnSig<'tcx> {
    #[serde(serialize_with = "vec__bound_variable_kind")]
    pub bound_vars: Vec<BoundVariableKind>,
    #[serde(with = "FnSigDef")]
    pub value: FnSig<'tcx>,
}

impl<'tcx> From<&Binder<'tcx, FnSig<'tcx>>> for Binder__FnSig<'tcx> {
    fn from(value: &Binder<'tcx, FnSig<'tcx>>) -> Self {
        Binder__FnSig {
            bound_vars: value.bound_vars().to_vec().clone(),
            value: value.skip_binder().clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(remote = "FnSig")]
pub struct FnSigDef<'tcx> {
    #[serde(with = "TysDef")]
    pub inputs_and_output: &'tcx List<Ty<'tcx>>,
    pub c_variadic: bool,
    #[serde(with = "UnsafetyDef")]
    pub unsafety: Unsafety,
    #[serde(with = "AbiDef")]
    pub abi: Abi,
}

#[derive(Serialize)]
#[serde(remote = "Unsafety")]
pub enum UnsafetyDef {
    Unsafe,
    Normal,
}

#[derive(Serialize)]
#[serde(remote = "Abi")]
pub enum AbiDef {
    Rust,
    C {
        unwind: bool,
    },
    Cdecl {
        unwind: bool,
    },
    Stdcall {
        unwind: bool,
    },
    Fastcall {
        unwind: bool,
    },
    Vectorcall {
        unwind: bool,
    },
    Thiscall {
        unwind: bool,
    },
    Aapcs {
        unwind: bool,
    },
    Win64 {
        unwind: bool,
    },
    SysV64 {
        unwind: bool,
    },
    PtxKernel,
    Msp430Interrupt,
    X86Interrupt,
    AmdGpuKernel,
    EfiApi,
    AvrInterrupt,
    AvrNonBlockingInterrupt,
    CCmseNonSecureCall,
    Wasm,
    System {
        unwind: bool,
    },
    RustIntrinsic,
    RustCall,
    PlatformIntrinsic,
    Unadjusted,
    RustCold,
    RiscvInterruptM,
    RiscvInterruptS,
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
#[serde(remote = "TraitRef")]
pub struct TraitRefDef<'tcx> {
    #[serde(with = "DefIdDef")]
    pub def_id: DefId,
    #[serde(with = "GenericArgsDef")]
    pub args: GenericArgsRef<'tcx>,
    #[serde(skip)]
    _use_trait_ref_new_instead: (),
}

pub struct GenericArgsDef;
impl  GenericArgsDef {
    pub fn serialize<'tcx, S>(value: &GenericArgs<'tcx>, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            vec__generic_arg(&value.to_vec(), s)
        }
}

fn vec__generic_arg<'tcx, S>(value: &Vec<GenericArg<'tcx>>, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    #[derive(Serialize)]
    struct Wrapper<'a, 'tcx>(#[serde(with = "GenericArgDef")] &'a GenericArg<'tcx>);
    serialize_custom_seq! { Wrapper, s, value }
}

#[derive(Serialize)]
#[serde(remote = "IntTy")]
pub enum IntTyDef {
    Isize,
    I8,
    I16,
    I32,
    I64,
    I128,
}

#[derive(Serialize)]
#[serde(remote = "UintTy")]
pub enum UintTyDef {
    Usize,
    U8,
    U16,
    U32,
    U64,
    U128,
}

#[derive(Serialize)]
#[serde(remote = "FloatTy")]
pub enum FloatTyDef {
    F32,
    F64,
}

pub struct AdtDefDef;
impl AdtDefDef {
    pub fn serialize<'tcx, S>(value: &AdtDef<'tcx>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        "TODO: AdtDef".serialize(s)
    }
}

#[derive(Serialize)]
#[serde(remote = "TypeAndMut")]
pub struct TypeAndMutDef<'tcx> {
    #[serde(with = "TyDef")]
    pub ty: Ty<'tcx>,
    #[serde(with = "MutabilityDef")]
    pub mutbl: Mutability,
}

#[derive(Serialize)]
#[serde(remote = "Mutability")]
pub enum MutabilityDef {
    Not,
    Mut,
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

pub struct RegionDef;
impl RegionDef {
    pub fn serialize<'tcx, S>(value: &Region<'tcx>, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            RegionKind__TyCtxt::from(&value.kind()).serialize(s)
        }
}

#[derive(Serialize)]
#[serde(remote = "BoundRegion")]
pub struct BoundRegionDef {
    #[serde(with = "BoundVarDef")]
    pub var: BoundVar,
    #[serde(with = "BoundRegionKindDef")]
    pub kind: BoundRegionKind,
}

#[derive(Serialize)]
pub enum RegionKind__TyCtxt<'tcx> {
    ReEarlyParam(#[serde(with = "EarlyParamRegionDef")] <TyCtxt<'tcx> as Interner>::EarlyParamRegion),
    ReBound(#[serde(skip)] DebruijnIndex, #[serde(with = "BoundRegionDef")] <TyCtxt<'tcx> as Interner>::BoundRegion),
    ReLateParam(#[serde(with = "LateParamRegionDef")] <TyCtxt<'tcx> as Interner>::LateParamRegion),
    ReStatic,
    ReVar(#[serde(with = "InferRegionDef")] <TyCtxt<'tcx> as Interner>::InferRegion),
    RePlaceholder(#[serde(with = "PlaceholderRegionDef")] <TyCtxt<'tcx> as Interner>::PlaceholderRegion),
    ReErased,
    ReError(#[serde(skip)] <TyCtxt<'tcx> as Interner>::ErrorGuaranteed),
}

impl<'tcx> From<&ir::RegionKind<TyCtxt<'tcx>>> for RegionKind__TyCtxt<'tcx> {
    fn from(value: &ir::RegionKind<TyCtxt<'tcx>>) -> Self {
        match value {
            RegionKind::ReEarlyParam(v) =>    RegionKind__TyCtxt::ReEarlyParam(v.clone()),
            RegionKind::ReBound(v1, v2) =>    RegionKind__TyCtxt::ReBound(v1.clone(), v2.clone()),
            RegionKind::ReLateParam(v) =>     RegionKind__TyCtxt::ReLateParam(v.clone()),
            RegionKind::ReStatic =>           RegionKind__TyCtxt::ReStatic,
            RegionKind::ReVar(v) =>           RegionKind__TyCtxt::ReVar(v.clone()),
            RegionKind::RePlaceholder(v) =>   RegionKind__TyCtxt::RePlaceholder(v.clone()),
            RegionKind::ReErased =>           RegionKind__TyCtxt::ReErased,
            RegionKind::ReError(v) =>         RegionKind__TyCtxt::ReError(v.clone()),
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

pub struct ConstDef;
impl ConstDef {
    pub fn serialize<'tcx, S>(value: &Const<'tcx>, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            ConstKind__TyCtxt::from(&value.kind()).serialize(s)
        }
}

#[derive(Serialize)]
pub enum ConstKind__TyCtxt<'tcx> {
    Param(#[serde(with = "ParamConstDef")] <TyCtxt<'tcx> as Interner>::ParamConst),
    Infer(#[serde(with = "InferConstDef")] InferConst),
    Bound(#[serde(skip)] DebruijnIndex, #[serde(with = "BoundConstDef")] <TyCtxt<'tcx> as Interner>::BoundConst),
    Placeholder(#[serde(with = "PlaceholderConstDef")] <TyCtxt<'tcx> as Interner>::PlaceholderConst),
    Unevaluated(#[serde(with = "AliasConstDef")] <TyCtxt<'tcx> as Interner>::AliasConst),
    Value(#[serde(with = "ValueConstDef")] <TyCtxt<'tcx> as Interner>::ValueConst),
    Error(#[serde(skip)] <TyCtxt<'tcx> as Interner>::ErrorGuaranteed),
    Expr(#[serde(with = "ExprConstDef")] <TyCtxt<'tcx> as Interner>::ExprConst),
}

impl<'tcx> From<&ir::ConstKind<TyCtxt<'tcx>>> for ConstKind__TyCtxt<'tcx> {
    fn from(value: &ir::ConstKind<TyCtxt<'tcx>>) -> Self {
        match value {
            ConstKind::Param(v)        =>ConstKind__TyCtxt::Param(v.clone()),
            ConstKind::Infer(v)        =>ConstKind__TyCtxt::Infer(v.clone()),
            ConstKind::Bound(v1, v2)        =>ConstKind__TyCtxt::Bound(v1.clone(), v2.clone()),
            ConstKind::Placeholder(v)  =>ConstKind__TyCtxt::Placeholder(v.clone()),
            ConstKind::Unevaluated(v)  =>ConstKind__TyCtxt::Unevaluated(v.clone()),
            ConstKind::Value(v)        =>ConstKind__TyCtxt::Value(v.clone()),
            ConstKind::Error(v)        =>ConstKind__TyCtxt::Error(v.clone()),
            ConstKind::Expr(v)         =>ConstKind__TyCtxt::Expr(v.clone()),
        }
    } 
}

#[derive(Serialize)]
#[serde(remote = "PlaceholderConst")]
struct PlaceholderConstDef {
    #[serde(with = "UniverseIndexDef")]
    pub universe: UniverseIndex,
    #[serde(with = "BoundVarDef")]
    pub bound: BoundVar,
}

#[derive(Serialize)]
#[serde(remote = "BoundVar")]
pub struct BoundVarDef {
    #[serde(skip)]
    pub(crate) private: u32,
}

#[derive(Serialize)]
#[serde(remote = "UniverseIndex")]
pub struct UniverseIndexDef {
    #[serde(skip)]
    pub(crate) private: u32,
}

pub struct GenericArgDef;
impl GenericArgDef {
    pub fn serialize<'tcx, S>(value: &GenericArg<'tcx>, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            GenericArgKindDef::serialize(&value.unpack(), s)
        }
}

#[derive(Serialize)]
#[serde(remote = "GenericArgKind")]
pub enum GenericArgKindDef<'tcx> {
    Lifetime(#[serde(with = "RegionDef")] Region<'tcx>),
    Type(#[serde(with = "TyDef")] Ty<'tcx>),
    Const(#[serde(with = "ConstDef")] Const<'tcx>),
}

#[derive(Serialize)]
#[serde(remote = "DefId")]
pub struct DefIdDef {
    #[serde(skip)]
    pub index: DefIndex,
    #[serde(skip)]
    pub krate: CrateNum,
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
#[serde(remote = "ConstData")]
pub struct ConstDataDef<'tcx> {
    #[serde(with = "TyDef")]
    pub ty: Ty<'tcx>,
    #[serde(skip)]
    pub kind: ConstKind<'tcx>,
}

#[derive(Serialize)]
#[serde(remote = "InferConst")]
pub enum InferConstDef {
    Var(#[serde(with = "ConstVidDef")] ConstVid),
    EffectVar(#[serde(with = "EffectVidDef")] EffectVid),
    Fresh(u32),
}

// FIXME: we can use getters to serialize the identifiers, 
// they're probably also not necessary

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
#[serde(remote = "UnevaluatedConst")]
pub struct UnevaluatedConstDef<'tcx> {
    #[serde(with = "DefIdDef")]
    pub def: DefId,
    #[serde(with = "GenericArgsDef")]
    pub args: GenericArgsRef<'tcx>,
}

pub struct AliasConstDef;
impl AliasConstDef {
    pub fn serialize<'tcx, S>(value: &UnevaluatedConst<'tcx>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        UnevaluatedConstDef::serialize(value, s)
    }
}

#[derive(Serialize)]
#[serde(remote = "ValTree")]
pub enum ValTreeDef<'tcx> {
    Leaf(#[serde(with = "ScalarIntDef")] ScalarInt),
    Branch(#[serde(serialize_with = "slice__val_tree")] &'tcx [ValTree<'tcx>]),
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

#[derive(Serialize)]
#[serde(remote = "InferTy")]
pub enum InferTyDef {
    TyVar(#[serde(with = "TyVidDef")] TyVid),
    IntVar(#[serde(with = "IntVidDef")] IntVid),
    FloatVar(#[serde(with = "FloatVidDef")] FloatVid),
    FreshTy(u32),
    FreshIntTy(u32),
    FreshFloatTy(u32),
}

#[derive(Serialize)]
#[serde(remote = "TyVid")]
pub struct TyVidDef {
    #[serde(skip)]
    private: u32,
}

#[derive(Serialize)]
#[serde(remote = "IntVid")]
pub struct IntVidDef {
    #[serde(skip)]
    private: u32,
}

#[derive(Serialize)]
#[serde(remote = "FloatVid")]
pub struct FloatVidDef {
    #[serde(skip)]
    private: u32,
}

#[derive(Serialize)]
#[serde(remote = "LateParamRegion")]
pub struct LateParamRegionDef {
    #[serde(with = "DefIdDef")]
    pub scope: DefId,
    #[serde(with = "BoundRegionKindDef")]
    pub bound_region: BoundRegionKind,
}

pub struct BoundConstDef;
impl BoundConstDef {
    pub fn serialize<'tcx, S>(value: &BoundVar, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        BoundVarDef::serialize(value, s)
    }
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

pub struct ValueConstDef;
impl ValueConstDef {
    pub fn serialize<'tcx, S>(value: &ValTree<'tcx>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        ValTreeDef::serialize(value, s)
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

pub struct ExprConstDef;
impl ExprConstDef {
    pub fn serialize<'tcx, S>(value: &Expr<'tcx>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        ExprDef::serialize(value, s)
    }
}

#[derive(Serialize)]
#[serde(remote = "Expr")]
pub enum ExprDef<'tcx> {
    Binop(#[serde(with = "BinOpDef")] BinOp, #[serde(with = "ConstDef")] Const<'tcx>, #[serde(with = "ConstDef")] Const<'tcx>),
    UnOp(#[serde(with = "UnOpDef")] UnOp, #[serde(with = "ConstDef")] Const<'tcx>),
    FunctionCall(#[serde(with = "ConstDef")] Const<'tcx>, #[serde(serialize_with = "list__const")] &'tcx List<Const<'tcx>>),
    Cast(#[serde(with = "CastKindDef")] CastKind, #[serde(with = "ConstDef")] Const<'tcx>, #[serde(with = "TyDef")] Ty<'tcx>),
}

fn list__const<'tcx, S>(value: &List<Const<'tcx>>, s: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
    #[derive(Serialize)]
    struct Wrapper<'tcx>(#[serde(with = "ConstDef")] Const<'tcx>);
    let mut seq = s.serialize_seq(Some(value.len()))?;
    for e in value {
        seq.serialize_element(&Wrapper(e))?;
    }
    seq.end()

}

#[derive(Serialize)]
#[serde(remote = "BinOp")]
pub enum BinOpDef {
    Add,
    AddUnchecked,
    Sub,
    SubUnchecked,
    Mul,
    MulUnchecked,
    Div,
    Rem,
    BitXor,
    BitAnd,
    BitOr,
    Shl,
    ShlUnchecked,
    Shr,
    ShrUnchecked,
    Eq,
    Lt,
    Le,
    Ne,
    Ge,
    Gt,
    Offset,
}

#[derive(Serialize)]
#[serde(remote = "UnOp")]
pub enum UnOpDef {
    Not,
    Neg,
}

#[derive(Serialize)]
#[serde(remote = "CastKind")]
pub enum CastKindDef {
    As,
    Use,
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

pub struct TermDef;
impl TermDef {
    pub fn serialize<'tcx, S>(value: &Term<'tcx>, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            TermKindDef::serialize(&value.unpack(), s)
        }
}

#[derive(Serialize)]
#[serde(remote = "TermKind")]
pub enum TermKindDef<'tcx> {
    Ty(#[serde(with = "TyDef")] Ty<'tcx>),
    Const(#[serde(with = "ConstDef")] Const<'tcx>),
}

#[derive(Serialize)]
#[serde(remote = "AliasRelationDirection")]
pub enum AliasRelationDirectionDef {
    Equate,
    Subtype,
}
