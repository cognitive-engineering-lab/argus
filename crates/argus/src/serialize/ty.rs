use std::num::*;

use rustc_hir::{def_id::DefId, Unsafety};
use rustc_infer::infer::type_variable::TypeVariableOriginKind;
use rustc_middle::ty::{self, *};
use rustc_span::symbol::{kw, Symbol};
use rustc_target::spec::abi::Abi;
use rustc_type_ir as ir;
use serde::{ser::SerializeSeq, Serialize};

use super::*;

pub struct TyDef;
impl TyDef {
  pub fn serialize<'tcx, S>(value: &Ty<'tcx>, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    TyKindDef::serialize(value.kind(), s)
  }
}

pub struct TysDef;
impl TysDef {
  pub fn serialize<'tcx, S>(
    value: &List<Ty<'tcx>>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    struct Wrapper<'tcx>(#[serde(with = "TyDef")] Ty<'tcx>);
    serialize_custom_seq! { Wrapper, s, value }
  }
}

pub struct TyKindDef;
impl TyKindDef {
  pub fn serialize<'tcx, S>(
    value: &TyKind<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    TyKind__TyCtxt::from(value).serialize(s)
  }
}

// NOTE: these types are modified from the original enum. They only store the information
// necessary for us to serialize the types, and the exchange is taken care of in the From impl.
// Everyting commented out is a TODO!
#[derive(Serialize)]
pub enum TyKind__TyCtxt<'tcx> {
  Bool,
  Char,
  Int(#[serde(with = "IntTyDef")] IntTy),
  Uint(#[serde(with = "UintTyDef")] UintTy),
  Float(#[serde(with = "FloatTyDef")] FloatTy),
  Adt(path::PathDefWithArgs<'tcx>),
  Str,
  Array(
    #[serde(with = "TyDef")] <TyCtxt<'tcx> as Interner>::Ty,
    #[serde(with = "ConstDef")] <TyCtxt<'tcx> as Interner>::Const,
  ),
  Slice(#[serde(with = "TyDef")] <TyCtxt<'tcx> as Interner>::Ty),
  RawPtr(#[serde(with = "TypeAndMutDef")] TypeAndMut<'tcx>),
  Ref(
    #[serde(with = "RegionDef")] <TyCtxt<'tcx> as Interner>::Region,
    #[serde(with = "TyDef")] <TyCtxt<'tcx> as Interner>::Ty,
    #[serde(with = "MutabilityDef")] Mutability,
  ),
  FnDef(path::PathDefWithArgs<'tcx>),
  Never,
  Tuple(#[serde(with = "TysDef")] <TyCtxt<'tcx> as Interner>::Tys),
  Placeholder(
    #[serde(with = "PlaceholderTyDef")]
    <TyCtxt<'tcx> as Interner>::PlaceholderTy,
  ),
  Infer(#[serde(with = "InferTyDef")] InferTy),
  Error,
  Foreign(#[serde(with = "DefIdDef")] <TyCtxt<'tcx> as Interner>::DefId),
  Closure(path::PathDefWithArgs<'tcx>),
  FnPtr(#[serde(with = "PolyFnSigDef")] <TyCtxt<'tcx> as Interner>::PolyFnSig),
  Param(#[serde(with = "ParamTyDef")] <TyCtxt<'tcx> as Interner>::ParamTy),
  Bound(
    #[serde(skip)] DebruijnIndex,
    #[serde(with = "BoundTyDef")] <TyCtxt<'tcx> as Interner>::BoundTy,
  ),
  // Alias(#[serde(with = "AliasKindDef")] AliasKind, #[serde(with = "AliasTyDef")] <TyCtxt<'tcx> as Interner>::AliasTy),
  // TODO: Dynamic(#[serde(with = "BoundExistentialPredicatesDef")] <TyCtxt<'tcx> as Interner>::BoundExistentialPredicates, #[serde(with = "RegionDef")] <TyCtxt<'tcx> as Interner>::Region, #[serde(with = "DynKindDef")] DynKind),
  // TODO: Coroutine(#[serde(with = "DefIdDef")] <TyCtxt<'tcx> as Interner>::DefId, #[serde(with = "GenericArgsDef")] <TyCtxt<'tcx> as Interner>::GenericArgs, #[serde(with = "MovabilityDef")] Movability),
  // TODO: CoroutineWitness(#[serde(with = "DefIdDef")] <TyCtxt<'tcx> as Interner>::DefId, #[serde(with = "GenericArgsDef")] <TyCtxt<'tcx> as Interner>::GenericArgs),
}

impl<'tcx> From<&ir::TyKind<TyCtxt<'tcx>>> for TyKind__TyCtxt<'tcx> {
  fn from(value: &ir::TyKind<TyCtxt<'tcx>>) -> Self {
    match value {
      ir::TyKind::Bool => TyKind__TyCtxt::Bool,
      ir::TyKind::Char => TyKind__TyCtxt::Char,
      ir::TyKind::Int(v) => TyKind__TyCtxt::Int(*v),
      ir::TyKind::Uint(v) => TyKind__TyCtxt::Uint(*v),
      ir::TyKind::Float(v) => TyKind__TyCtxt::Float(*v),
      ir::TyKind::Str => TyKind__TyCtxt::Str,
      ir::TyKind::Adt(def, args) => {
        TyKind__TyCtxt::Adt(path::PathDefWithArgs::new(def.did(), args))
      }
      ir::TyKind::Array(ty, sz) => TyKind__TyCtxt::Array(*ty, *sz),
      ir::TyKind::Slice(ty) => TyKind__TyCtxt::Slice(*ty),
      ir::TyKind::Ref(r, ty, mutbl) => TyKind__TyCtxt::Ref(*r, *ty, *mutbl),
      ir::TyKind::FnDef(def_id, args) => {
        TyKind__TyCtxt::FnDef(path::PathDefWithArgs::new(*def_id, args))
      }
      ir::TyKind::Never => TyKind__TyCtxt::Never,
      ir::TyKind::Tuple(tys) => TyKind__TyCtxt::Tuple(tys.clone()),
      ir::TyKind::Placeholder(v) => TyKind__TyCtxt::Placeholder(*v),
      ir::TyKind::Error(_) => TyKind__TyCtxt::Error,
      ir::TyKind::Infer(v) => TyKind__TyCtxt::Infer(*v),
      ir::TyKind::RawPtr(tam) => TyKind__TyCtxt::RawPtr(*tam),
      ir::TyKind::Foreign(d) => TyKind__TyCtxt::Foreign(*d),
      ir::TyKind::Closure(def_id, args) => {
        TyKind__TyCtxt::Closure(path::PathDefWithArgs::new(*def_id, args))
      }
      ir::TyKind::FnPtr(v) => TyKind__TyCtxt::FnPtr(v.clone()),
      ir::TyKind::Param(param_ty) => TyKind__TyCtxt::Param(param_ty.clone()),
      ir::TyKind::Bound(dji, bound_ty) => {
        TyKind__TyCtxt::Bound(*dji, bound_ty.clone())
      }

      // TODO(gavinleroy)
      ir::TyKind::Alias(_k, _aty) => todo!("alias"),
      ir::TyKind::Dynamic(_bep, _r, _dy_kind) => todo!("dynamic"),
      ir::TyKind::Coroutine(_def_id, _args) => todo!("coroutine"),
      ir::TyKind::CoroutineWitness(_def_id, _args) => {
        todo!("coroutine-witness")
      }
    }
  }
}

pub struct PlaceholderTyDef;
impl PlaceholderTyDef {
  pub fn serialize<'tcx, S>(
    value: &Placeholder<BoundTy>,
    s: S,
  ) -> Result<S::Ok, S::Error>
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
  fn from(value: &Placeholder<BoundTy>) -> Self {
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
  fn from(value: &Placeholder<BoundRegion>) -> Self {
    Placeholder__BoundRegion {
      universe: value.universe.clone(),
      bound: value.bound.clone(),
    }
  }
}

pub struct PolyFnSigDef;
impl PolyFnSigDef {
  pub fn serialize<'tcx, S>(
    value: &Binder<'tcx, FnSig<'tcx>>,
    s: S,
  ) -> Result<S::Ok, S::Error>
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
  C { unwind: bool },
  Cdecl { unwind: bool },
  Stdcall { unwind: bool },
  Fastcall { unwind: bool },
  Vectorcall { unwind: bool },
  Thiscall { unwind: bool },
  Aapcs { unwind: bool },
  Win64 { unwind: bool },
  SysV64 { unwind: bool },
  PtxKernel,
  Msp430Interrupt,
  X86Interrupt,
  AmdGpuKernel,
  EfiApi,
  AvrInterrupt,
  AvrNonBlockingInterrupt,
  CCmseNonSecureCall,
  Wasm,
  System { unwind: bool },
  RustIntrinsic,
  RustCall,
  PlatformIntrinsic,
  Unadjusted,
  RustCold,
  RiscvInterruptM,
  RiscvInterruptS,
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

// #[derive(Serialize)]
// #[serde(remote = "AliasTy")]
// pub struct AliasTyDef<'tcx> {
//     #[serde(with = "GenericArgsDef")]
//     pub args: GenericArgsRef<'tcx>,
//     #[serde(with = "DefIdDef")]
//     pub def_id: DefId,
//     #[serde(skip)]
//     _use_alias_ty_new_instead: (),
// }

pub struct AliasTyDef;
impl AliasTyDef {
  pub fn serialize<'tcx, S>(
    value: &AliasTy<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    path::PathDefWithArgs::new(value.def_id, value.args).serialize(s)
  }
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
  BrNamed(
    #[serde(with = "DefIdDef")] DefId,
    #[serde(with = "SymbolDef")] Symbol,
  ),
  BrEnv,
}

#[derive(Serialize)]
#[serde(remote = "BoundTyKind")]
pub enum BoundTyKindDef {
  Anon,
  Param(
    #[serde(with = "DefIdDef")] DefId,
    #[serde(with = "SymbolDef")] Symbol,
  ),
}

pub struct GenericArgsDef;
impl GenericArgsDef {
  pub fn serialize<'tcx, S>(
    value: &GenericArgs<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    vec__generic_arg(&value.to_vec(), s)
  }
}

fn vec__generic_arg<'tcx, S>(
  value: &Vec<GenericArg<'tcx>>,
  s: S,
) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  #[derive(Serialize)]
  struct Wrapper<'a, 'tcx>(
    #[serde(with = "GenericArgDef")] &'a GenericArg<'tcx>,
  );
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
  pub fn serialize<'tcx, S>(
    _value: &AdtDef<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
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

// pub struct TypeAndMutDef;
// impl TypeAndMutDef {
//     pub fn serialize<'tcx, S>(value: &TypeAndMut<TyCtxt<'tcx>>, s: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         #[derive(Serialize)]
//         struct Wrapper {
//             #[serde(with = "TyDef")]
//             pub ty: &<TyCtxt<'tcx> as Interner>::Ty<'tcx>,
//             #[serde(with = "MutabilityDef")]
//             pub mutbl: Mutability,
//         }
//         Wrapper { ty: &value.ty, mutbl: value.mutbl }.serialize(s)
//     }
// }

#[derive(Serialize)]
#[serde(remote = "Mutability")]
pub enum MutabilityDef {
  Not,
  Mut,
}

pub struct RegionDef;
impl RegionDef {
  pub fn serialize<'tcx, S>(
    value: &Region<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
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
  ReEarlyParam(
    #[serde(with = "EarlyParamRegionDef")]
    <TyCtxt<'tcx> as Interner>::EarlyParamRegion,
  ),
  ReBound(
    #[serde(skip)] DebruijnIndex,
    #[serde(with = "BoundRegionDef")] <TyCtxt<'tcx> as Interner>::BoundRegion,
  ),
  ReLateParam(
    #[serde(with = "LateParamRegionDef")]
    <TyCtxt<'tcx> as Interner>::LateParamRegion,
  ),
  ReStatic,
  ReVar(
    #[serde(with = "InferRegionDef")] <TyCtxt<'tcx> as Interner>::InferRegion,
  ),
  RePlaceholder(
    #[serde(with = "PlaceholderRegionDef")]
    <TyCtxt<'tcx> as Interner>::PlaceholderRegion,
  ),
  ReErased,
  ReError(#[serde(skip)] <TyCtxt<'tcx> as Interner>::ErrorGuaranteed),
}

impl<'tcx> From<&ir::RegionKind<TyCtxt<'tcx>>> for RegionKind__TyCtxt<'tcx> {
  fn from(value: &ir::RegionKind<TyCtxt<'tcx>>) -> Self {
    match value {
      RegionKind::ReEarlyParam(v) => {
        RegionKind__TyCtxt::ReEarlyParam(v.clone())
      }
      RegionKind::ReBound(v1, v2) => {
        RegionKind__TyCtxt::ReBound(v1.clone(), v2.clone())
      }
      RegionKind::ReLateParam(v) => RegionKind__TyCtxt::ReLateParam(v.clone()),
      RegionKind::ReStatic => RegionKind__TyCtxt::ReStatic,
      RegionKind::ReVar(v) => RegionKind__TyCtxt::ReVar(v.clone()),
      RegionKind::RePlaceholder(v) => {
        RegionKind__TyCtxt::RePlaceholder(v.clone())
      }
      RegionKind::ReErased => RegionKind__TyCtxt::ReErased,
      RegionKind::ReError(v) => RegionKind__TyCtxt::ReError(v.clone()),
    }
  }
}

#[derive(Serialize)]
#[serde(remote = "BoundVar")]
pub struct BoundVarDef {
  #[serde(skip)]
  pub(crate) private: u32,
}

pub struct GenericArgDef;
impl GenericArgDef {
  pub fn serialize<'tcx, S>(
    value: &GenericArg<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
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

pub struct InferTyDef;
impl InferTyDef {
  pub fn serialize<'tcx, S>(value: &InferTy, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    struct InferTyVar {
      #[serde(with = "TyVidDef")]
      ty_var: TyVid,
    }
    #[derive(Serialize)]
    struct InferVar {
      infer_var: String,
    }
    // For TyVars we need to look up the type in the dynamic inference context.
    // I've let that dispatch to the TyVidDef to keep it in one place, all other
    // inference variables can be serialized with their `ToString` impl.
    if let InferTy::TyVar(ty_var) = value {
      log::debug!("Serializing infer var {:?}", value);
      InferTyVar { ty_var: *ty_var }.serialize(s)
    } else {
      InferVar {
        infer_var: value.to_string(),
      }
      .serialize(s)
    }
  }
}

// TODO:  when I don't know what to do, I just return "Unresolved", this
// should correspond to some '_' character in the frontend. I'm certainly
// doing something wrong with the inference variables, but I don't know what (yet).
pub struct TyVidDef;
impl TyVidDef {
  pub fn serialize<'a, 'tcx: 'a, S>(
    value: &TyVid,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let infcx = get_dynamic_ctx();
    let ty_vid = *value;

    let Ok(ty) = infcx.probe_ty_var(ty_vid) else {
      // If the type variable isn't resolved, what to do?
      log::debug!("ERROR UniverseIdx probe_ty_var {ty_vid:?}");
      return "Unresolved".serialize(s);
    };

    let Some(var_origin) = infcx.type_var_origin(ty) else {
      // If the type is not a type variable, None is returned.
      // In this case we can just serialize the type.
      log::debug!("ERR no var_origin {ty:?}");
      return TyDef::serialize(&ty, s);
    };

    if let TypeVariableOriginKind::TypeParameterDefinition(name, def_id) = var_origin.kind
            && name != kw::SelfUpper
            && !var_origin.span.from_expansion()
        {
            let generics = infcx.tcx.generics_of(infcx.tcx.parent(def_id));
            let idx = generics.param_def_id_to_index(infcx.tcx, def_id).unwrap();
            let generic_param_def = generics.param_at(idx as usize, infcx.tcx);
            if let ty::GenericParamDefKind::Type { synthetic: true, .. } = generic_param_def.kind {
                "Unresolved".serialize(s)
            } else {
                SymbolDef::serialize(&name, s)
            }
        } else {
            "Unresolved".serialize(s)
        }
  }
}

#[derive(Serialize)]
#[serde(remote = "LateParamRegion")]
pub struct LateParamRegionDef {
  #[serde(with = "DefIdDef")]
  pub scope: DefId,
  #[serde(with = "BoundRegionKindDef")]
  pub bound_region: BoundRegionKind,
}

pub fn goal__predicate_def<'tcx, S>(
  value: &Goal<'tcx, Predicate<'tcx>>,
  s: S,
) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
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
      param_env: value.param_env,
    }
  }
}

pub struct ParamEnvDef;
impl ParamEnvDef {
  pub fn serialize<'tcx, S>(
    value: &ParamEnv<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
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
  pub fn serialize<'tcx, S>(
    value: &Predicate<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
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
  pub fn serialize<'tcx, S>(
    value: &Binder<'tcx, ClauseKind<'tcx>>,
    s: S,
  ) -> Result<S::Ok, S::Error>
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
    }
    .serialize(s)
  }
}

#[derive(Serialize)]
pub struct Binder__PredicateKind<'tcx> {
  #[serde(serialize_with = "vec__bound_variable_kind")]
  pub bound_vars: Vec<BoundVariableKind>,
  #[serde(with = "PredicateKindDef")]
  pub value: ir::PredicateKind<TyCtxt<'tcx>>,
}

impl<'tcx> From<&Binder<'tcx, ir::PredicateKind<TyCtxt<'tcx>>>>
  for Binder__PredicateKind<'tcx>
{
  fn from(value: &Binder<'tcx, ir::PredicateKind<TyCtxt<'tcx>>>) -> Self {
    Binder__PredicateKind {
      bound_vars: value.bound_vars().to_vec(),
      value: value.skip_binder().clone(),
    }
  }
}

fn vec__bound_variable_kind<S>(
  value: &Vec<BoundVariableKind>,
  s: S,
) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  #[derive(Serialize)]
  struct Wrapper<'a>(
    #[serde(with = "BoundVariableKindDef")] &'a BoundVariableKind,
  );
  serialize_custom_seq! { Wrapper, s, value }
}

pub struct BoundExistentialPredicatesDef;
impl BoundExistentialPredicatesDef {
  pub fn serialize<'tcx, S>(
    value: &List<Binder<'tcx, ExistentialPredicate<'tcx>>>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    struct Wrapper<'tcx>(
      #[serde(serialize_with = "binder__existential_predicate")]
      Binder<'tcx, ExistentialPredicate<'tcx>>,
    );
    serialize_custom_seq! { Wrapper, s, value }
  }
}

fn binder__existential_predicate<'tcx, S>(
  value: &Binder<'tcx, ExistentialPredicate<'tcx>>,
  s: S,
) -> Result<S::Ok, S::Error>
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

impl<'tcx> From<&Binder<'tcx, ExistentialPredicate<'tcx>>>
  for Binder__ExistentialPredicate<'tcx>
{
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
  Projection(
    #[serde(with = "ExistentialProjectionDef")] ExistentialProjection<'tcx>,
  ),
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
  Clause(
    #[serde(serialize_with = "clause_kind__ty_ctxt")]
    ir::ClauseKind<TyCtxt<'tcx>>,
  ),
  ObjectSafe(#[serde(with = "DefIdDef")] DefId),
  Subtype(#[serde(with = "SubtypePredicateDef")] SubtypePredicate<'tcx>),
  Coerce(#[serde(with = "CoercePredicateDef")] CoercePredicate<'tcx>),
  ConstEquate(
    #[serde(with = "ConstDef")] Const<'tcx>,
    #[serde(with = "ConstDef")] Const<'tcx>,
  ),
  Ambiguous,
  NormalizesTo(#[serde(with = "NormalizesToDef")] NormalizesTo<'tcx>),
  AliasRelate(
    #[serde(with = "TermDef")] Term<'tcx>,
    #[serde(with = "TermDef")] Term<'tcx>,
    #[serde(with = "AliasRelationDirectionDef")] AliasRelationDirection,
  ),
  // ClosureKind(#[serde(with = "DefIdDef")] DefId, #[serde(with = "GenericArgsDef")] GenericArgsRef<'tcx>, #[serde(with = "ClosureKindDef")] ClosureKind),
}

#[derive(Serialize)]
#[serde(remote = "NormalizesTo")]
pub struct NormalizesToDef<'tcx> {
  #[serde(with = "AliasTyDef")]
  pub alias: AliasTy<'tcx>,
  #[serde(with = "TermDef")]
  pub term: Term<'tcx>,
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

fn clause_kind__ty_ctxt<'tcx, S>(
  value: &ir::ClauseKind<TyCtxt<'tcx>>,
  s: S,
) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  ClauseKind__TyCtxt::from(value).serialize(s)
}

#[derive(Serialize)]
pub enum ClauseKind__TyCtxt<'tcx> {
  Trait(
    #[serde(with = "TraitPredicateDef")]
    <TyCtxt<'tcx> as Interner>::TraitPredicate,
  ),
  RegionOutlives(
    #[serde(with = "RegionOutlivesPredicateDef")]
    <TyCtxt<'tcx> as Interner>::RegionOutlivesPredicate,
  ),
  TypeOutlives(
    #[serde(with = "TypeOutlivesPredicateDef")]
    <TyCtxt<'tcx> as Interner>::TypeOutlivesPredicate,
  ),
  Projection(
    #[serde(with = "ProjectionPredicateDef")]
    <TyCtxt<'tcx> as Interner>::ProjectionPredicate,
  ),
  ConstArgHasType(
    #[serde(with = "ConstDef")] <TyCtxt<'tcx> as Interner>::Const,
    #[serde(with = "TyDef")] <TyCtxt<'tcx> as Interner>::Ty,
  ),
  WellFormed(
    #[serde(with = "GenericArgDef")] <TyCtxt<'tcx> as Interner>::GenericArg,
  ),
  ConstEvaluatable(
    #[serde(with = "ConstDef")] <TyCtxt<'tcx> as Interner>::Const,
  ),
}

impl<'tcx> From<&ir::ClauseKind<TyCtxt<'tcx>>> for ClauseKind__TyCtxt<'tcx> {
  fn from(value: &ir::ClauseKind<TyCtxt<'tcx>>) -> Self {
    match value {
      ClauseKind::Trait(v) => ClauseKind__TyCtxt::Trait(v.clone()),
      ClauseKind::RegionOutlives(v) => {
        ClauseKind__TyCtxt::RegionOutlives(v.clone())
      }
      ClauseKind::TypeOutlives(v) => {
        ClauseKind__TyCtxt::TypeOutlives(v.clone())
      }
      ClauseKind::Projection(v) => ClauseKind__TyCtxt::Projection(v.clone()),
      ClauseKind::ConstArgHasType(v1, v2) => {
        ClauseKind__TyCtxt::ConstArgHasType(v1.clone(), v2.clone())
      }
      ClauseKind::WellFormed(v) => ClauseKind__TyCtxt::WellFormed(v.clone()),
      ClauseKind::ConstEvaluatable(v) => {
        ClauseKind__TyCtxt::ConstEvaluatable(v.clone())
      }
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

#[derive(Debug, Serialize)]
pub struct TraitRefPrintOnlyTraitPathDefWrapper<'tcx>(
  #[serde(with = "TraitRefPrintOnlyTraitPathDef")] pub TraitRef<'tcx>,
);

pub struct TraitRefPrintOnlyTraitPathDef;
impl TraitRefPrintOnlyTraitPathDef {
  pub fn serialize<'tcx, S>(
    value: &TraitRef<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    log::debug!("Serializing TraitRef[path only] {:#?}", value);
    path::PathDefWithArgs::new(value.def_id, value.args).serialize(s)
  }
}

pub struct TraitRefDef;
impl TraitRefDef {
  pub fn serialize<'tcx, S>(
    value: &TraitRef<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
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
    }
    .serialize(s)
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
  pub fn serialize<'tcx, S>(
    value: &OutlivesPredicate<Region<'tcx>, Region<'tcx>>,
    s: S,
  ) -> Result<S::Ok, S::Error>
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

impl<'tcx> From<&OutlivesPredicate<Region<'tcx>, Region<'tcx>>>
  for OutlivesPredicate__Region__Region<'tcx>
{
  fn from(value: &OutlivesPredicate<Region<'tcx>, Region<'tcx>>) -> Self {
    OutlivesPredicate__Region__Region {
      a: value.0.clone(),
      b: value.1.clone(),
    }
  }
}

pub struct TypeOutlivesPredicateDef;
impl TypeOutlivesPredicateDef {
  pub fn serialize<'tcx, S>(
    value: &OutlivesPredicate<Ty<'tcx>, Region<'tcx>>,
    s: S,
  ) -> Result<S::Ok, S::Error>
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

impl<'tcx> From<&OutlivesPredicate<Ty<'tcx>, Region<'tcx>>>
  for OutlivesPredicate__Ty__Region<'tcx>
{
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
    S: serde::Serializer,
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
    S: serde::Serializer,
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
  pub fn serialize<'tcx, S>(
    value: &Placeholder<BoundRegion>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Placeholder__BoundRegion::from(value).serialize(s)
  }
}

pub fn slice__val_tree<'tcx, S>(
  value: &[ValTree<'tcx>],
  s: S,
) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
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
