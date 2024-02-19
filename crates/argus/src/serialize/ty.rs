use rustc_data_structures::fx::FxIndexMap;
use rustc_hir::{self as hir, def::DefKind, def_id::DefId, LangItem, Unsafety};
use rustc_infer::traits::{ObligationCause, PredicateObligation};
use rustc_middle::{traits::util::supertraits_for_pretty_printing, ty};
use rustc_span::symbol::{kw, Symbol};
use rustc_target::spec::abi::Abi;
use serde::Serialize;
use smallvec::SmallVec;
#[cfg(feature = "testing")]
use ts_rs::TS;

use super::{r#const::*, term::*, *};

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "Ty"))]
pub struct TyDef<'tcx>(
  #[serde(with = "TyKindDef")]
  #[cfg_attr(feature = "testing", ts(type = "TyKind"))]
  &'tcx ty::TyKind<'tcx>,
);

impl<'tcx> TyDef<'tcx> {
  pub fn serialize<S>(value: &ty::Ty<'tcx>, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self(value.kind()).serialize(s)
  }
}

pub type TysDef = Slice__TyDef;
pub struct Slice__TyDef;
impl Slice__TyDef {
  pub fn serialize<'tcx, S>(
    value: &[ty::Ty<'tcx>],
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    struct Wrapper<'a, 'tcx: 'a>(#[serde(with = "TyDef")] &'a ty::Ty<'tcx>);
    serialize_custom_seq! { Wrapper, s, value }
  }
}

pub struct Option__TyDef;
impl Option__TyDef {
  pub fn serialize<'tcx, S>(
    value: &Option<ty::Ty<'tcx>>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    match value {
      None => s.serialize_none(),
      Some(ty) => TyDef::serialize(ty, s),
    }
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "TyKind"))]
pub enum TyKindDef<'tcx> {
  Bool,
  Char,
  Int(
    #[serde(with = "IntTyDef")]
    #[cfg_attr(feature = "testing", ts(type = "IntTy"))]
    ty::IntTy,
  ),
  Uint(
    #[serde(with = "UintTyDef")]
    #[cfg_attr(feature = "testing", ts(type = "UintTy"))]
    ty::UintTy,
  ),
  Float(
    #[serde(with = "FloatTyDef")]
    #[cfg_attr(feature = "testing", ts(type = "FloatTy"))]
    ty::FloatTy,
  ),
  Adt(path::PathDefWithArgs<'tcx>),
  Str,
  Array(
    #[serde(with = "TyDef")]
    #[cfg_attr(feature = "testing", ts(type = "Ty"))]
    ty::Ty<'tcx>,
    #[serde(with = "ConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "Const"))]
    ty::Const<'tcx>,
  ),
  Slice(
    #[serde(with = "TyDef")]
    #[cfg_attr(feature = "testing", ts(type = "Ty"))]
    ty::Ty<'tcx>,
  ),
  RawPtr(
    #[serde(with = "TypeAndMutDef")]
    #[cfg_attr(feature = "testing", ts(type = "TypeAndMut"))]
    ty::TypeAndMut<'tcx>,
  ),
  Ref(
    #[serde(with = "RegionDef")]
    #[cfg_attr(feature = "testing", ts(type = "Region"))]
    ty::Region<'tcx>,
    #[serde(with = "TyDef")]
    #[cfg_attr(feature = "testing", ts(type = "Ty"))]
    ty::Ty<'tcx>,
    #[serde(with = "MutabilityDef")]
    #[cfg_attr(feature = "testing", ts(type = "Mutability"))]
    ty::Mutability,
  ),
  FnDef(FnDef<'tcx>),
  FnPtr(
    #[serde(with = "PolyFnSigDef")]
    #[cfg_attr(feature = "testing", ts(type = "PolyFnSig"))]
    ty::PolyFnSig<'tcx>,
  ),
  Never,
  Tuple(
    #[serde(with = "Slice__TyDef")]
    #[cfg_attr(feature = "testing", ts(type = "Ty[]"))]
    &'tcx ty::List<ty::Ty<'tcx>>,
  ),
  Placeholder(
    #[serde(with = "PlaceholderTyDef")]
    #[cfg_attr(feature = "testing", ts(type = "PlaceholderBoundTy"))]
    ty::Placeholder<ty::BoundTy>,
  ),
  Infer(
    #[serde(with = "InferTyDef")]
    #[cfg_attr(feature = "testing", ts(type = "InferTy"))]
    ty::InferTy,
  ),
  Error,
  Foreign(path::PathDefNoArgs<'tcx>),
  Closure(path::PathDefWithArgs<'tcx>),
  Param(
    #[serde(with = "ParamTyDef")]
    #[cfg_attr(feature = "testing", ts(type = "ParamTy"))]
    ty::ParamTy,
  ),
  Bound(BoundTyDef),
  Alias(AliasTyKindDef<'tcx>),
  Dynamic(DynamicTyKindDef<'tcx>),
  Coroutine(CoroutineTyKindDef<'tcx>),
  CoroutineWitness(CoroutineWitnessTyKindDef<'tcx>),
}

impl<'tcx> From<&ty::TyKind<'tcx>> for TyKindDef<'tcx> {
  fn from(value: &ty::TyKind<'tcx>) -> Self {
    match value {
      ty::TyKind::Bool => Self::Bool,
      ty::TyKind::Char => Self::Char,
      ty::TyKind::Int(v) => Self::Int(*v),
      ty::TyKind::Uint(v) => Self::Uint(*v),
      ty::TyKind::Float(v) => Self::Float(*v),
      ty::TyKind::Str => Self::Str,
      ty::TyKind::Adt(def, args) => {
        Self::Adt(path::PathDefWithArgs::new(def.did(), args))
      }
      ty::TyKind::Array(ty, sz) => Self::Array(*ty, *sz),
      ty::TyKind::Slice(ty) => Self::Slice(*ty),
      ty::TyKind::Ref(r, ty, mutbl) => Self::Ref(*r, *ty, *mutbl),
      ty::TyKind::FnDef(def_id, args) => Self::FnDef(FnDef::new(*def_id, args)),
      ty::TyKind::Never => Self::Never,
      ty::TyKind::Tuple(tys) => Self::Tuple(tys),
      ty::TyKind::Placeholder(v) => Self::Placeholder(*v),
      ty::TyKind::Error(_) => Self::Error,
      ty::TyKind::Infer(v) => Self::Infer(*v),
      ty::TyKind::RawPtr(tam) => Self::RawPtr(*tam),
      ty::TyKind::Foreign(d) => Self::Foreign(path::PathDefNoArgs::new(*d)),
      ty::TyKind::Closure(def_id, args) => {
        Self::Closure(path::PathDefWithArgs::new(*def_id, args))
      }
      ty::TyKind::FnPtr(v) => Self::FnPtr(v.clone()),
      ty::TyKind::Param(param_ty) => Self::Param(param_ty.clone()),
      ty::TyKind::Bound(dji, bound_ty) => {
        Self::Bound(BoundTyDef::new(*dji, *bound_ty))
      }
      ty::TyKind::Alias(k, aty) => Self::Alias(AliasTyKindDef::new(*k, *aty)),
      ty::TyKind::Dynamic(bep, r, dy_kind) => {
        Self::Dynamic(DynamicTyKindDef::new(bep, r, *dy_kind))
      }
      ty::TyKind::Coroutine(def_id, args) => {
        Self::Coroutine(CoroutineTyKindDef::new(*def_id, args))
      }
      ty::TyKind::CoroutineWitness(def_id, args) => {
        Self::CoroutineWitness(CoroutineWitnessTyKindDef::new(*def_id, args))
      }
    }
  }
}

impl<'tcx> TyKindDef<'tcx> {
  pub fn serialize<S>(value: &ty::TyKind<'tcx>, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    TyKindDef::from(value).serialize(s)
  }
}

// -----------------------------------
// Alias types

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "AliasTyKind"))]
#[serde(tag = "type")]
pub enum AliasTyKindDef<'tcx> {
  OpaqueImpl {
    data: OpaqueImpl<'tcx>,
  },
  AliasTy {
    #[serde(with = "AliasTyDef")]
    #[cfg_attr(feature = "testing", ts(type = "AliasTy"))]
    data: ty::AliasTy<'tcx>,
  },
  DefPath {
    data: path::PathDefWithArgs<'tcx>,
  },
}

impl<'tcx> AliasTyKindDef<'tcx> {
  pub fn new(kind: ty::AliasKind, ty: ty::AliasTy<'tcx>) -> Self {
    let infcx = get_dynamic_ctx();
    match (kind, ty) {
      (
        ty::AliasKind::Projection
        | ty::AliasKind::Inherent
        | ty::AliasKind::Weak,
        ref data,
      ) => {
        if !(infcx.should_print_verbose() || with_no_queries())
          && infcx.tcx.is_impl_trait_in_trait(data.def_id)
        {
          // CHANGE: return this.pretty_print_opaque_impl_type(data.def_id, data.args);
          Self::OpaqueImpl {
            data: OpaqueImpl::new(data.def_id, data.args),
          }
        } else {
          // CHANGE: p!(print(data))
          Self::AliasTy { data: *data }
        }
      }
      (ty::AliasKind::Opaque, ty::AliasTy { def_id, args, .. }) => {
        // We use verbose printing in 'NO_QUERIES' mode, to
        // avoid needing to call `predicates_of`. This should
        // only affect certain debug messages (e.g. messages printed
        // from `rustc_middle::ty` during the computation of `tcx.predicates_of`),
        // and should have no effect on any compiler output.
        // [Unless `-Zverbose-internals` is used, e.g. in the output of
        // `tests/ui/nll/ty-outlives/impl-trait-captures.rs`, for
        // example.]
        if infcx.should_print_verbose() {
          // FIXME(eddyb) print this with `print_def_path`.
          // CHANGE: p!(write("Opaque({:?}, {})", def_id, args.print_as_list()));
          // return Ok(())
          // NOTE: I'm taking the risk of using print_def_path here
          // as indicated by the above comment. If things break, look here.
          return Self::DefPath {
            data: path::PathDefWithArgs::new(def_id, args),
          };
        }

        let parent = infcx.tcx.parent(def_id);
        match infcx.tcx.def_kind(parent) {
          DefKind::TyAlias | DefKind::AssocTy => {
            // NOTE: I know we should check for NO_QUERIES here, but it's alright.
            // `type_of` on a type alias or assoc type should never cause a cycle.
            if let ty::Alias(ty::Opaque, ty::AliasTy { def_id: d, .. }) =
              *infcx.tcx.type_of(parent).instantiate_identity().kind()
            {
              if d == def_id {
                // If the type alias directly starts with the `impl` of the
                // opaque type we're printing, then skip the `::{opaque#1}`.
                // CHANGE: p!(print_def_path(parent, args));
                // return Ok(())
                return Self::DefPath {
                  data: path::PathDefWithArgs::new(parent, args),
                };
              }
            }
            // Complex opaque type, e.g. `type Foo = (i32, impl Debug);`
            // CHANGE: p!(print_def_path(def_id, args));
            // return Ok(())
            return Self::DefPath {
              data: path::PathDefWithArgs::new(def_id, args),
            };
          }
          _ => {
            if with_no_queries() {
              // CHANGE: p!(print_def_path(def_id, &[]));
              // return Ok(())
              Self::DefPath {
                data: path::PathDefWithArgs::new(def_id, &[]),
              }
            } else {
              // CHANGE: return this.pretty_print_opaque_impl_type(def_id, args);
              Self::OpaqueImpl {
                data: OpaqueImpl::new(def_id, args),
              }
            }
          }
        }
      }
    }
  }
}

// -----------------------------------
// Dynamic types

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "DynamicTyKind"))]
pub struct DynamicTyKindDef<'tcx> {
  predicates: PolyExistentialPredicatesDef<'tcx>,

  #[serde(with = "RegionDef")]
  #[cfg_attr(feature = "testing", ts(type = "Region"))]
  region: ty::Region<'tcx>,

  #[serde(with = "DynKindDef")]
  #[cfg_attr(feature = "testing", ts(type = "DynKind"))]
  kind: ty::DynKind,
}

impl<'tcx> DynamicTyKindDef<'tcx> {
  pub fn new(
    predicates: &ty::List<ty::PolyExistentialPredicate<'tcx>>,
    region: &ty::Region<'tcx>,
    kind: ty::DynKind,
  ) -> Self {
    Self {
      predicates: PolyExistentialPredicatesDef::new(predicates),
      region: *region,
      kind,
    }
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(
  feature = "testing",
  ts(export, rename = "PolyExistentialPredicates")
)]
#[serde(rename_all = "camelCase")]
pub struct PolyExistentialPredicatesDef<'tcx> {
  #[serde(skip_serializing_if = "Option::is_none")]
  data: Option<path::PathDefNoArgs<'tcx>>,
  auto_traits: Vec<path::PathDefNoArgs<'tcx>>,
}

impl<'tcx> PolyExistentialPredicatesDef<'tcx> {
  pub fn new(
    predicates: &ty::List<ty::PolyExistentialPredicate<'tcx>>,
  ) -> Self {
    let data = predicates.principal().map(|principal| {
      // TODO: how to deal with binders
      let principal = principal.skip_binder();

      // TODO: see pretty_print_dyn_existential where
      // they do some wonky special casing and "re-sugaring"...

      path::PathDefNoArgs::new(principal.def_id)
    });
    let auto_traits: Vec<_> = predicates
      .auto_traits()
      .map(|def_id| path::PathDefNoArgs::new(def_id))
      .collect::<Vec<_>>();

    Self { data, auto_traits }
  }

  pub fn serialize<S>(
    value: &ty::List<ty::PolyExistentialPredicate<'tcx>>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self::new(value).serialize(s)
  }
}

// -----------------------------------
// Coroutine definitions

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "CoroutineTyKind"))]
pub struct CoroutineTyKindDef<'tcx> {
  path: path::PathDefWithArgs<'tcx>,
  #[serde(with = "MovabilityDef")]
  #[cfg_attr(feature = "testing", ts(type = "Movability"))]
  movability: ty::Movability,

  #[serde(with = "TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  upvar_tys: ty::Ty<'tcx>,

  #[serde(with = "TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  witness: ty::Ty<'tcx>,
  should_print_movability: bool,
}

impl<'tcx> CoroutineTyKindDef<'tcx> {
  pub fn new(
    def_id: DefId,
    args: &'tcx ty::List<ty::GenericArg<'tcx>>,
  ) -> Self {
    let infcx = get_dynamic_ctx();
    let tcx = infcx.tcx;

    let coroutine_kind = tcx.coroutine_kind(def_id).unwrap();
    let upvar_tys = args.as_coroutine().tupled_upvars_ty();
    let witness = args.as_coroutine().witness();
    let movability = coroutine_kind.movability();

    Self {
      path: path::PathDefWithArgs::new(def_id, &*args),
      movability,
      upvar_tys,
      witness,
      should_print_movability: matches!(
        coroutine_kind,
        hir::CoroutineKind::Coroutine(_)
      ),
    }
  }
}

// -----------------------------------
// Coroutine witness definitions

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "CoroutineWitnessTyKind"))]
pub struct CoroutineWitnessTyKindDef<'tcx>(path::PathDefWithArgs<'tcx>);
impl<'tcx> CoroutineWitnessTyKindDef<'tcx> {
  pub fn new(
    def_id: DefId,
    args: &'tcx ty::List<ty::GenericArg<'tcx>>,
  ) -> Self {
    Self(path::PathDefWithArgs::new(def_id, args))
  }
}

// -----------------------------------
// Function definitions

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "FnDef"))]
pub struct FnDef<'tcx> {
  #[serde(with = "PolyFnSigDef")]
  #[cfg_attr(feature = "testing", ts(type = "PolyFnSig"))]
  sig: ty::PolyFnSig<'tcx>,
  path: path::ValuePathWithArgs<'tcx>,
}

impl<'tcx> FnDef<'tcx> {
  pub fn new(def_id: DefId, args: &'tcx [ty::GenericArg<'tcx>]) -> Self {
    let infcx = get_dynamic_ctx();
    let sig = infcx.tcx.fn_sig(def_id).instantiate(infcx.tcx, args);
    Self {
      sig,
      path: path::ValuePathWithArgs::new(def_id, args),
    }
  }
}

// -----------------------------------
// Placeholder definitions

#[derive(Serialize)]
#[serde(tag = "type")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "PlaceholderBoundTy"))]
pub enum PlaceholderTyDef {
  Named {
    #[serde(with = "SymbolDef")]
    #[cfg_attr(feature = "testing", ts(type = "Symbol"))]
    data: Symbol,
  },
  Anon,
}

impl PlaceholderTyDef {
  pub fn serialize<'tcx, S>(
    value: &ty::Placeholder<ty::BoundTy>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let serialize_kind = match value.bound.kind {
      ty::BoundTyKind::Anon => Self::Anon,
      ty::BoundTyKind::Param(_, name) => Self::Named { data: name },
    };

    serialize_kind.serialize(s)
  }
}

// -----------------------------------
// Function signature definitions

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "PolyFnSig"))]
pub struct Binder__FnSigDef<'tcx> {
  #[serde(with = "FnSigDef")]
  #[cfg_attr(feature = "testing", ts(type = "FnSig"))]
  value: ty::FnSig<'tcx>,

  #[serde(with = "Slice__BoundVariableKindDef")]
  #[cfg_attr(feature = "testing", ts(type = "BoundVariableKind[]"))]
  bound_vars: &'tcx ty::List<ty::BoundVariableKind>,
}

type PolyFnSigDef<'tcx> = Binder__FnSigDef<'tcx>;

impl<'tcx> Binder__FnSigDef<'tcx> {
  pub fn new(value: &ty::Binder<'tcx, ty::FnSig<'tcx>>) -> Self {
    Self {
      bound_vars: value.bound_vars(),
      value: value.skip_binder(),
    }
  }

  pub fn serialize<S>(
    value: &ty::Binder<'tcx, ty::FnSig<'tcx>>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self::new(value).serialize(s)
  }
}

#[derive(Serialize)]
#[serde(remote = "ty::FnSig")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "FnSig"))]
pub struct FnSigDef<'tcx> {
  #[serde(with = "Slice__TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty[]"))]
  pub inputs_and_output: &'tcx ty::List<ty::Ty<'tcx>>,
  pub c_variadic: bool,

  #[serde(with = "UnsafetyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Unsafety"))]
  pub unsafety: Unsafety,

  #[serde(with = "AbiDef")]
  #[cfg_attr(feature = "testing", ts(type = "Abi"))]
  pub abi: Abi,
}

#[derive(Serialize)]
#[serde(remote = "Unsafety")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "Unsafety"))]
pub enum UnsafetyDef {
  Unsafe,
  Normal,
}

#[derive(Serialize)]
#[serde(remote = "Abi")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "Abi"))]
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
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "DynKind"))]
#[serde(remote = "ty::DynKind")]
pub enum DynKindDef {
  Dyn,
  DynStar,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "Movability"))]
#[serde(remote = "ty::Movability")]
pub enum MovabilityDef {
  Static,
  Movable,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "AliasTy"))]
#[serde(tag = "type")]
pub enum AliasTyDef<'tcx> {
  Inherent { data: path::AliasPath<'tcx> },
  PathDef { data: path::PathDefWithArgs<'tcx> },
}

impl<'tcx> AliasTyDef<'tcx> {
  pub fn new(value: &ty::AliasTy<'tcx>) -> Self {
    let cx = get_dynamic_ctx();
    if let DefKind::Impl { of_trait: false } =
      cx.tcx.def_kind(cx.tcx.parent(value.def_id))
    {
      // CHANGE: p!(pretty_print_inherent_projection(self))
      Self::Inherent {
        data: path::AliasPath::new(*value),
      }
    } else {
      // CHANGE: p!(print_def_path(self.def_id, self.args));
      Self::PathDef {
        data: path::PathDefWithArgs::new(value.def_id, value.args),
      }
    }
  }

  pub fn serialize<S>(
    value: &ty::AliasTy<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self::new(value).serialize(s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "BoundTy"))]
#[serde(tag = "type")]
pub enum BoundTyDef {
  Named {
    #[serde(with = "SymbolDef")]
    #[cfg_attr(feature = "testing", ts(type = "Symbol"))]
    data: Symbol,
  },
  Bound {
    data: BoundVariable,
  },
}

impl BoundTyDef {
  pub fn new(debruijn: ty::DebruijnIndex, ty: ty::BoundTy) -> Self {
    match ty.kind {
      ty::BoundTyKind::Anon => Self::Bound {
        data: BoundVariable::new(debruijn, ty.var),
      },
      ty::BoundTyKind::Param(_, name) => Self::Named { data: name },
    }
  }
}

// --------------------------------------------
// --------------------------------------------
// TODO: the DefId's here need to be dealt with

#[derive(Serialize)]
#[serde(remote = "ty::BoundVariableKind")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "BoundVariableKind"))]
pub enum BoundVariableKindDef {
  Ty(
    #[serde(with = "BoundTyKindDef")]
    #[cfg_attr(feature = "testing", ts(type = "any"))]
    ty::BoundTyKind,
  ),
  Region(
    #[serde(with = "BoundRegionKindDef")]
    #[cfg_attr(feature = "testing", ts(type = "BoundRegionKind"))]
    ty::BoundRegionKind,
  ),
  Const,
}

pub struct Slice__BoundVariableKindDef;
impl Slice__BoundVariableKindDef {
  fn serialize<S>(
    value: &[ty::BoundVariableKind],
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    struct Wrapper<'a>(
      #[serde(with = "BoundVariableKindDef")] &'a ty::BoundVariableKind,
    );
    serialize_custom_seq! { Wrapper, s, value }
  }
}

#[derive(Serialize)]
#[serde(remote = "ty::BoundRegionKind")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "BoundRegionKind"))]
pub enum BoundRegionKindDef {
  BrAnon,
  BrNamed(
    #[serde(skip)] DefId,
    #[serde(with = "SymbolDef")]
    #[cfg_attr(feature = "testing", ts(type = "Symbol"))]
    Symbol,
  ),
  BrEnv,
}

#[derive(Serialize)]
#[serde(remote = "ty::BoundTyKind")]
// #[cfg_attr(feature = "testing", derive(TS))]
// #[cfg_attr(feature = "testing", ts(export, rename = "BoundTyKind"))]
pub enum BoundTyKindDef {
  Anon,
  Param(#[serde(skip)] DefId, #[serde(skip)] Symbol),
}

// ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
// ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
// ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

#[derive(Serialize)]
#[serde(remote = "ty::IntTy")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "IntTy"))]
pub enum IntTyDef {
  Isize,
  I8,
  I16,
  I32,
  I64,
  I128,
}

#[derive(Serialize)]
#[serde(remote = "ty::UintTy")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "UintTy"))]
pub enum UintTyDef {
  Usize,
  U8,
  U16,
  U32,
  U64,
  U128,
}

#[derive(Serialize)]
#[serde(remote = "ty::FloatTy")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "FloatTy"))]
pub enum FloatTyDef {
  F32,
  F64,
}

#[derive(Serialize)]
#[serde(remote = "ty::TypeAndMut")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "TypeAndMut"))]
pub struct TypeAndMutDef<'tcx> {
  #[serde(with = "TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  pub ty: ty::Ty<'tcx>,

  #[serde(with = "MutabilityDef")]
  #[cfg_attr(feature = "testing", ts(type = "Mutability"))]
  pub mutbl: ty::Mutability,
}

#[derive(Serialize)]
#[serde(remote = "ty::Mutability")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "Mutability"))]
pub enum MutabilityDef {
  Not,
  Mut,
}

// NOTE: this follows the code for "concise printout" code from print::pretty,
// but this isn't really all the information you'd want to diagnose a region error.
// A stretch goal for Argus would be to explain regions in some way similar
// to `note_and_explain_region`.
// TODO: we should use some sort of "region highlight mode"
// see: <https://doc.rust-lang.org/stable/nightly-rustc/rustc_middle/ty/print/pretty/struct.RegionHighlightMode.html>
// to differentiate regions in the types, I guess not necessary now.
#[derive(Serialize)]
#[serde(tag = "type")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "Region"))]
pub enum RegionDef {
  Named {
    #[serde(with = "SymbolDef")]
    #[cfg_attr(feature = "testing", ts(type = "Symbol"))]
    data: Symbol,
  },
  Anonymous,
  Static,
}

impl<'tcx> RegionDef {
  pub fn new(value: &ty::Region<'tcx>) -> Self {
    let region = value;
    match **region {
      ty::ReEarlyParam(ref data) if data.name != kw::Empty => {
        Self::Named { data: data.name }
      }
      ty::ReBound(_, ty::BoundRegion { kind: br, .. })
      | ty::ReLateParam(ty::LateParamRegion {
        bound_region: br, ..
      })
      | ty::RePlaceholder(ty::Placeholder {
        bound: ty::BoundRegion { kind: br, .. },
        ..
      }) if let ty::BrNamed(_, name) = br
        && br.is_named() =>
      {
        Self::Named { data: name }
      }
      ty::ReStatic => Self::Static,

      // XXX: the catch all case is for those from above with guards, in the
      // future if we expand the capabilities of the region printing this will
      // need to change.
      ty::ReVar(_) | ty::ReErased | ty::ReError(_) | _ => Self::Anonymous,
    }
  }
}

impl RegionDef {
  pub fn serialize<'tcx, S>(
    value: &ty::Region<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self::new(value).serialize(s)
  }
}

pub struct Slice__RegionDef;
impl Slice__RegionDef {
  pub fn serialize<'tcx, S>(
    value: &[ty::Region<'tcx>],
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    struct Wrapper<'a, 'tcx: 'a>(
      #[serde(with = "RegionDef")] &'a ty::Region<'tcx>,
    );
    serialize_custom_seq! { Wrapper, s, value }
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "GenericArg"))]
pub struct GenericArgDef<'tcx>(
  #[serde(with = "GenericArgKindDef")]
  #[cfg_attr(feature = "testing", ts(type = "GenericArgKind"))]
  ty::GenericArgKind<'tcx>,
);

impl<'tcx> GenericArgDef<'tcx> {
  pub fn serialize<S>(
    value: &ty::GenericArg<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self(value.unpack()).serialize(s)
  }
}

pub struct Slice__GenericArgDef;
impl Slice__GenericArgDef {
  pub fn serialize<'tcx, S>(
    value: &[ty::GenericArg<'tcx>],
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    struct Wrapper<'a, 'tcx: 'a>(
      #[serde(with = "GenericArgDef")] &'a ty::GenericArg<'tcx>,
    );
    serialize_custom_seq! { Wrapper, s, value }
  }
}

#[derive(Serialize)]
#[serde(remote = "ty::GenericArgKind")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "GenericArgKind"))]
pub enum GenericArgKindDef<'tcx> {
  Lifetime(
    #[serde(with = "RegionDef")]
    #[cfg_attr(feature = "testing", ts(type = "Region"))]
    ty::Region<'tcx>,
  ),
  Type(
    #[serde(with = "TyDef")]
    #[cfg_attr(feature = "testing", ts(type = "Ty"))]
    ty::Ty<'tcx>,
  ),
  Const(
    #[serde(with = "ConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "Const"))]
    ty::Const<'tcx>,
  ),
}

// TODO: gavinleroy
#[derive(Serialize)]
#[serde(tag = "type")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "InferTy"))]
pub enum InferTyDef<'tcx> {
  IntVar,
  FloatVar,
  // TODO: We should also include source information
  #[serde(rename_all = "camelCase")]
  Named {
    #[serde(with = "SymbolDef")]
    #[cfg_attr(feature = "testing", ts(type = "Symbol"))]
    name: Symbol,
    path_def: path::PathDefNoArgs<'tcx>,
  },
  Unresolved,
}

impl<'tcx> InferTyDef<'tcx> {
  pub fn new(value: &ty::InferTy) -> Self {
    use rustc_infer::infer::type_variable::TypeVariableOriginKind::TypeParameterDefinition;

    let infcx = get_dynamic_ctx();
    let tcx = infcx.tcx;

    let ty = ty::Ty::new_infer(tcx, *value);

    if let Some(type_origin) = infcx.type_var_origin(ty)
      && let TypeParameterDefinition(name, def_id) = type_origin.kind
    {
      Self::Named {
        name,
        path_def: path::PathDefNoArgs::new(def_id),
      }
    } else {
      match value {
        // TODO: can we do any better in these cases??
        ty::InferTy::TyVar(_) | ty::InferTy::FreshTy(_) => Self::Unresolved,
        ty::InferTy::IntVar(_) | ty::InferTy::FreshIntTy(_) => Self::IntVar,
        ty::InferTy::FloatVar(_) | ty::InferTy::FreshFloatTy(_) => {
          Self::FloatVar
        }
      }
    }
  }

  pub fn serialize<S>(value: &ty::InferTy, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self::new(value).serialize(s)
  }
}

// ------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(remote = "PredicateObligation")]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "PredicateObligation"))]
pub struct PredicateObligationDef<'tcx> {
  #[serde(skip)]
  pub cause: ObligationCause<'tcx>,

  #[serde(with = "ParamEnvDef")]
  #[cfg_attr(feature = "testing", ts(type = "ParamEnv"))]
  pub param_env: ty::ParamEnv<'tcx>,

  #[serde(with = "PredicateDef")]
  #[cfg_attr(feature = "testing", ts(type = "Predicate"))]
  pub predicate: ty::Predicate<'tcx>,

  #[serde(skip)]
  pub recursion_depth: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "GoalPredicate"))]
pub struct Goal__PredicateDef<'tcx> {
  #[serde(with = "PredicateDef")]
  #[cfg_attr(feature = "testing", ts(type = "Predicate"))]
  pub predicate: ty::Predicate<'tcx>,

  #[serde(with = "ParamEnvDef")]
  #[cfg_attr(feature = "testing", ts(type = "ParamEnv"))]
  pub param_env: ty::ParamEnv<'tcx>,
}

impl<'tcx> Goal__PredicateDef<'tcx> {
  pub fn serialize<S>(
    value: &Goal<'tcx, ty::Predicate<'tcx>>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self {
      predicate: value.predicate,
      param_env: value.param_env,
    }
    .serialize(s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "ParamEnv"))]
pub struct ParamEnvDef<'a, 'tcx: 'a>(
  #[serde(with = "Slice__ClauseDef")]
  #[cfg_attr(feature = "testing", ts(type = "Clause[]"))]
  &'a ty::List<ty::Clause<'tcx>>,
);

impl<'tcx> ParamEnvDef<'_, 'tcx> {
  pub fn serialize<S>(
    value: &ty::ParamEnv<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self(value.caller_bounds()).serialize(s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "Predicate"))]
pub struct PredicateDef<'tcx>(
  #[serde(with = "Binder__PredicateKind")]
  #[cfg_attr(feature = "testing", ts(type = "PolyPredicateKind"))]
  ty::Binder<'tcx, ty::PredicateKind<'tcx>>,
);

impl<'tcx> PredicateDef<'tcx> {
  pub fn serialize<S>(
    value: &ty::Predicate<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self(value.kind()).serialize(s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "PolyPredicateKind"))]
pub struct Binder__PredicateKind<'tcx> {
  #[serde(with = "Slice__BoundVariableKindDef")]
  #[cfg_attr(feature = "testing", ts(type = "BoundVariableKind[]"))]
  pub bound_vars: Vec<ty::BoundVariableKind>,

  #[serde(with = "PredicateKindDef")]
  #[cfg_attr(feature = "testing", ts(type = "PredicateKind"))]
  pub value: ty::PredicateKind<'tcx>,
}

impl<'tcx> Binder__PredicateKind<'tcx> {
  pub fn serialize<S>(
    value: &ty::Binder<'tcx, ty::PredicateKind<'tcx>>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Binder__PredicateKind {
      bound_vars: value.bound_vars().to_vec(),
      value: value.skip_binder().clone(),
    }
    .serialize(s)
  }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "PolyClauseKind"))]
pub struct Binder__ClauseKindDef<'tcx> {
  #[serde(with = "Slice__BoundVariableKindDef")]
  #[cfg_attr(feature = "testing", ts(type = "BoundVariableKind[]"))]
  pub bound_vars: Vec<ty::BoundVariableKind>,

  #[serde(with = "ClauseKindDef")]
  #[cfg_attr(feature = "testing", ts(type = "ClauseKind"))]
  pub value: ty::ClauseKind<'tcx>,
}

impl<'tcx> Binder__ClauseKindDef<'tcx> {
  pub fn serialize<S>(
    value: &ty::Binder<'tcx, ty::ClauseKind<'tcx>>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self {
      bound_vars: value.bound_vars().to_vec(),
      value: value.skip_binder(),
    }
    .serialize(s)
  }
}

#[derive(Serialize)]
#[serde(remote = "ty::PredicateKind")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "PredicateKind"))]
pub enum PredicateKindDef<'tcx> {
  Clause(
    #[serde(with = "ClauseKindDef")]
    #[cfg_attr(feature = "testing", ts(type = "ClauseKind"))]
    ty::ClauseKind<'tcx>,
  ),
  ObjectSafe(
    #[serde(with = "path::PathDefNoArgs")]
    #[cfg_attr(feature = "testing", ts(type = "PathDefNoArgs"))]
    DefId,
  ),
  Subtype(
    #[serde(with = "SubtypePredicateDef")]
    #[cfg_attr(feature = "testing", ts(type = "SubtypePredicate"))]
    ty::SubtypePredicate<'tcx>,
  ),
  Coerce(
    #[serde(with = "CoercePredicateDef")]
    #[cfg_attr(feature = "testing", ts(type = "CoercePredicate"))]
    ty::CoercePredicate<'tcx>,
  ),
  ConstEquate(
    #[serde(with = "ConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "Const"))]
    ty::Const<'tcx>,
    #[serde(with = "ConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "Const"))]
    ty::Const<'tcx>,
  ),
  Ambiguous,
  NormalizesTo(
    #[serde(with = "NormalizesToDef")]
    #[cfg_attr(feature = "testing", ts(type = "NormalizesTo"))]
    ty::NormalizesTo<'tcx>,
  ),
  AliasRelate(
    #[serde(with = "TermDef")]
    #[cfg_attr(feature = "testing", ts(type = "Term"))]
    ty::Term<'tcx>,
    #[serde(with = "TermDef")]
    #[cfg_attr(feature = "testing", ts(type = "Term"))]
    ty::Term<'tcx>,
    #[serde(with = "AliasRelationDirectionDef")]
    #[cfg_attr(feature = "testing", ts(type = "AliasRelationDirection"))]
    ty::AliasRelationDirection,
  ),
}

#[derive(Serialize)]
#[serde(remote = "ty::NormalizesTo")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "NormalizesTo"))]
pub struct NormalizesToDef<'tcx> {
  #[serde(with = "AliasTyDef")]
  #[cfg_attr(feature = "testing", ts(type = "AliasTy"))]
  pub alias: ty::AliasTy<'tcx>,

  #[serde(with = "TermDef")]
  #[cfg_attr(feature = "testing", ts(type = "Term"))]
  pub term: ty::Term<'tcx>,
}

#[derive(Serialize)]
#[serde(remote = "ty::ClosureKind")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "ClosureKind"))]
pub enum ClosureKindDef {
  Fn,
  FnMut,
  FnOnce,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "Clause"))]
pub struct ClauseDef<'tcx>(
  #[serde(with = "Binder__ClauseKindDef")]
  #[cfg_attr(feature = "testing", ts(type = "PolyClauseKind"))]
  ty::Binder<'tcx, ty::ClauseKind<'tcx>>,
);

impl<'tcx> ClauseDef<'tcx> {
  fn serialize<S>(value: &ty::Clause<'tcx>, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self(value.kind()).serialize(s)
  }
}

pub struct Slice__ClauseDef;
impl Slice__ClauseDef {
  pub fn serialize<'tcx, S>(
    value: &[ty::Clause<'tcx>],
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    struct Wrapper<'a, 'tcx: 'a>(
      #[serde(with = "ClauseDef")] &'a ty::Clause<'tcx>,
    );
    serialize_custom_seq! { Wrapper, s, value }
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "ClauseKind"))]
pub enum ClauseKindDef<'tcx> {
  Trait(
    #[serde(with = "TraitPredicateDef")]
    #[cfg_attr(feature = "testing", ts(type = "TraitPredicate"))]
    ty::TraitPredicate<'tcx>,
  ),
  RegionOutlives(RegionOutlivesRegionDef<'tcx>),
  TypeOutlives(TyOutlivesRegionDef<'tcx>),
  Projection(
    #[serde(with = "ProjectionPredicateDef")]
    #[cfg_attr(feature = "testing", ts(type = "ProjectionPredicate"))]
    ty::ProjectionPredicate<'tcx>,
  ),
  ConstArgHasType(
    #[serde(with = "ConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "Const"))]
    ty::Const<'tcx>,
    #[serde(with = "TyDef")]
    #[cfg_attr(feature = "testing", ts(type = "Ty"))]
    ty::Ty<'tcx>,
  ),
  WellFormed(
    #[serde(with = "GenericArgDef")]
    #[cfg_attr(feature = "testing", ts(type = "GenericArg"))]
    ty::GenericArg<'tcx>,
  ),
  ConstEvaluatable(
    #[serde(with = "ConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "Const"))]
    ty::Const<'tcx>,
  ),
}

impl<'tcx> ClauseKindDef<'tcx> {
  fn serialize<S>(value: &ty::ClauseKind<'tcx>, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self::from(value).serialize(s)
  }
}

impl<'tcx> From<&ty::ClauseKind<'tcx>> for ClauseKindDef<'tcx> {
  fn from(value: &ty::ClauseKind<'tcx>) -> Self {
    match value {
      ty::ClauseKind::Trait(v) => Self::Trait(v.clone()),
      ty::ClauseKind::RegionOutlives(v) => {
        Self::RegionOutlives(RegionOutlivesRegionDef::new(v))
      }
      ty::ClauseKind::TypeOutlives(v) => {
        Self::TypeOutlives(TyOutlivesRegionDef::new(v))
      }
      ty::ClauseKind::Projection(v) => Self::Projection(v.clone()),
      ty::ClauseKind::ConstArgHasType(v1, v2) => {
        Self::ConstArgHasType(v1.clone(), v2.clone())
      }
      ty::ClauseKind::WellFormed(v) => Self::WellFormed(v.clone()),
      ty::ClauseKind::ConstEvaluatable(v) => Self::ConstEvaluatable(v.clone()),
    }
  }
}

#[derive(Serialize)]
#[serde(remote = "ty::SubtypePredicate")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "SubtypePredicate"))]
pub struct SubtypePredicateDef<'tcx> {
  pub a_is_expected: bool,

  #[serde(with = "TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  pub a: ty::Ty<'tcx>,

  #[serde(with = "TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  pub b: ty::Ty<'tcx>,
}

#[derive(Serialize)]
#[serde(remote = "ty::TraitPredicate")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "TraitPredicate"))]
pub struct TraitPredicateDef<'tcx> {
  #[serde(with = "TraitRefDef")]
  #[cfg_attr(feature = "testing", ts(type = "TraitRef"))]
  pub trait_ref: ty::TraitRef<'tcx>,

  #[serde(with = "ImplPolarityDef")]
  #[cfg_attr(feature = "testing", ts(type = "ImplPolarity"))]
  pub polarity: ty::ImplPolarity,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(
  feature = "testing",
  ts(export, rename = "TraitRefPrintOnlyTraitPath")
)]
pub struct TraitRefPrintOnlyTraitPathDef<'tcx>(path::PathDefWithArgs<'tcx>);
impl<'tcx> TraitRefPrintOnlyTraitPathDef<'tcx> {
  pub fn new(value: &ty::TraitRef<'tcx>) -> Self {
    Self(path::PathDefWithArgs::new(value.def_id, value.args))
  }

  pub fn serialize<S>(
    value: &ty::TraitRef<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self::new(value).serialize(s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "TraitRef"))]
pub struct TraitRefDef<'tcx> {
  #[serde(with = "TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  pub self_ty: ty::Ty<'tcx>,

  pub trait_path: TraitRefPrintOnlyTraitPathDef<'tcx>,
}

impl<'tcx> TraitRefDef<'tcx> {
  pub fn serialize<S>(
    value: &ty::TraitRef<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self {
      self_ty: value.self_ty(),
      trait_path: TraitRefPrintOnlyTraitPathDef::new(value),
    }
    .serialize(s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "ImplPolarity"))]
#[serde(remote = "ty::ImplPolarity")]
pub enum ImplPolarityDef {
  Positive,
  Negative,
  Reservation,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "RegionOutlivesRegion"))]
pub struct RegionOutlivesRegionDef<'tcx> {
  #[serde(with = "RegionDef")]
  #[cfg_attr(feature = "testing", ts(type = "Region"))]
  pub a: ty::Region<'tcx>,

  #[serde(with = "RegionDef")]
  #[cfg_attr(feature = "testing", ts(type = "Region"))]
  pub b: ty::Region<'tcx>,
}

impl<'tcx> RegionOutlivesRegionDef<'tcx> {
  pub fn new(
    value: &ty::OutlivesPredicate<ty::Region<'tcx>, ty::Region<'tcx>>,
  ) -> Self {
    Self {
      a: value.0.clone(),
      b: value.1.clone(),
    }
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "TyOutlivesRegion"))]
pub struct TyOutlivesRegionDef<'tcx> {
  #[serde(with = "TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  pub a: ty::Ty<'tcx>,

  #[serde(with = "RegionDef")]
  #[cfg_attr(feature = "testing", ts(type = "Region"))]
  pub b: ty::Region<'tcx>,
}

impl<'tcx> TyOutlivesRegionDef<'tcx> {
  pub fn new(
    value: &ty::OutlivesPredicate<ty::Ty<'tcx>, ty::Region<'tcx>>,
  ) -> Self {
    Self {
      a: value.0.clone(),
      b: value.1.clone(),
    }
  }
}

#[derive(Serialize)]
#[serde(remote = "ty::ProjectionPredicate")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "ProjectionPredicate"))]
pub struct ProjectionPredicateDef<'tcx> {
  #[serde(with = "AliasTyDef")]
  #[cfg_attr(feature = "testing", ts(type = "AliasTy"))]
  pub projection_ty: ty::AliasTy<'tcx>,

  #[serde(with = "TermDef")]
  #[cfg_attr(feature = "testing", ts(type = "Term"))]
  pub term: ty::Term<'tcx>,
}

#[derive(Serialize)]
#[serde(remote = "ty::UniverseIndex")]
pub struct UniverseIndexDef {
  #[serde(skip)]
  pub(crate) private: u32,
}

#[derive(Serialize)]
#[serde(remote = "ty::CoercePredicate")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "CoercePredicate"))]
pub struct CoercePredicateDef<'tcx> {
  #[serde(with = "TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  pub a: ty::Ty<'tcx>,

  #[serde(with = "TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  pub b: ty::Ty<'tcx>,
}

#[derive(Serialize)]
#[serde(remote = "ty::ParamTy")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "ParamTy"))]
pub struct ParamTyDef {
  #[serde(skip)]
  pub index: u32,

  #[serde(with = "SymbolDef")]
  #[cfg_attr(feature = "testing", ts(type = "Symbol"))]
  pub name: Symbol,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS), ts(export, rename = "Symbol"))]
pub struct SymbolDef<'a>(&'a str);

impl<'a> SymbolDef<'a> {
  pub fn serialize<'tcx, S>(value: &'a Symbol, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    SymbolDef(value.as_str()).serialize(s)
  }
}

pub struct Slice__SymbolDef;
impl Slice__SymbolDef {
  pub fn serialize<'tcx, S>(value: &[Symbol], s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    struct Wrapper<'a>(#[serde(with = "SymbolDef")] &'a Symbol);
    serialize_custom_seq! { Wrapper, s, value }
  }
}

pub struct Option__SymbolDef;
impl Option__SymbolDef {
  pub fn serialize<'tcx, S>(
    value: &Option<Symbol>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    match value {
      None => s.serialize_none(),
      Some(sym) => SymbolDef::serialize(sym, s),
    }
  }
}

#[derive(Serialize)]
#[serde(remote = "ty::AliasRelationDirection")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "AliasRelationDirection"))]
pub enum AliasRelationDirectionDef {
  Equate,
  Subtype,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum BoundVariable {}

impl BoundVariable {
  pub fn new(debruijn_idx: ty::DebruijnIndex, var: ty::BoundVar) -> Self {
    todo!("bound variables are not yet possible")
  }
}

// --------------------------------------------------------
// Opaque impl types

#[derive(Default, Debug)]
pub struct OpaqueFnEntry<'tcx> {
  // The trait ref is already stored as a key, so just track if we have it as a real predicate
  has_fn_once: bool,
  fn_mut_trait_ref: Option<ty::PolyTraitRef<'tcx>>,
  fn_trait_ref: Option<ty::PolyTraitRef<'tcx>>,
  return_ty: Option<ty::Binder<'tcx, ty::Term<'tcx>>>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
#[serde(rename_all = "camelCase")]
pub struct OpaqueImpl<'tcx> {
  fn_traits: Vec<FnTrait<'tcx>>,
  traits: Vec<Trait<'tcx>>,
  #[serde(with = "Slice__RegionDef")]
  #[cfg_attr(feature = "testing", ts(type = "Region[]"))]
  lifetimes: Vec<ty::Region<'tcx>>,
  has_sized_bound: bool,
  has_negative_sized_bound: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct FnTrait<'tcx> {
  #[serde(with = "Slice__TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty[]"))]
  params: Vec<ty::Ty<'tcx>>,

  #[serde(with = "Option__TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty | undefined"))]
  ret_ty: Option<ty::Ty<'tcx>>,

  kind: FnTraitKind,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct Trait<'tcx> {
  #[serde(with = "ImplPolarityDef")]
  #[cfg_attr(feature = "testing", ts(type = "ImplPolarity"))]
  polarity: ty::ImplPolarity,
  #[cfg_attr(feature = "testing", ts(type = "DefinedPath"))]
  trait_name: TraitRefPrintOnlyTraitPathDef<'tcx>,
  #[serde(with = "Slice__GenericArgDef")]
  #[cfg_attr(feature = "testing", ts(type = "GenericArg[]"))]
  own_args: &'tcx [ty::GenericArg<'tcx>],
  assoc_args: Vec<AssocItemDef<'tcx>>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "AssocItem"))]
pub struct AssocItemDef<'tcx> {
  #[serde(with = "SymbolDef")]
  #[cfg_attr(feature = "testing", ts(type = "Symbol"))]
  name: Symbol,
  #[serde(with = "super::term::TermDef")]
  #[cfg_attr(feature = "testing", ts(type = "Term"))]
  term: ty::Term<'tcx>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum FnTraitKind {
  FnMut,
  Fn,
  FnOnce,
}

impl<'tcx> OpaqueImpl<'tcx> {
  fn insert_trait_and_projection(
    tcx: ty::TyCtxt<'tcx>,
    trait_ref: ty::PolyTraitRef<'tcx>,
    polarity: ty::ImplPolarity,
    proj_ty: Option<(DefId, ty::Binder<'tcx, ty::Term<'tcx>>)>,
    traits: &mut FxIndexMap<
      (ty::PolyTraitRef<'tcx>, ty::ImplPolarity),
      FxIndexMap<DefId, ty::Binder<'tcx, ty::Term<'tcx>>>,
    >,
    fn_traits: &mut FxIndexMap<ty::PolyTraitRef<'tcx>, OpaqueFnEntry<'tcx>>,
  ) {
    let trait_def_id = trait_ref.def_id();

    // If our trait_ref is FnOnce or any of its children, project it onto the parent FnOnce
    // super-trait ref and record it there.
    // We skip negative Fn* bounds since they can't use parenthetical notation anyway.
    if polarity == ty::ImplPolarity::Positive
      && let Some(fn_once_trait) = tcx.lang_items().fn_once_trait()
    {
      // If we have a FnOnce, then insert it into
      if trait_def_id == fn_once_trait {
        let entry = fn_traits.entry(trait_ref).or_default();
        // Optionally insert the return_ty as well.
        if let Some((_, ty)) = proj_ty {
          entry.return_ty = Some(ty);
        }
        entry.has_fn_once = true;
        return;
      } else if Some(trait_def_id) == tcx.lang_items().fn_mut_trait() {
        let super_trait_ref = supertraits_for_pretty_printing(tcx, trait_ref)
          .find(|super_trait_ref| super_trait_ref.def_id() == fn_once_trait)
          .unwrap();

        fn_traits
          .entry(super_trait_ref)
          .or_default()
          .fn_mut_trait_ref = Some(trait_ref);
        return;
      } else if Some(trait_def_id) == tcx.lang_items().fn_trait() {
        let super_trait_ref = supertraits_for_pretty_printing(tcx, trait_ref)
          .find(|super_trait_ref| super_trait_ref.def_id() == fn_once_trait)
          .unwrap();

        fn_traits.entry(super_trait_ref).or_default().fn_trait_ref =
          Some(trait_ref);
        return;
      }
    }

    // Otherwise, just group our traits and projection types.
    traits
      .entry((trait_ref, polarity))
      .or_default()
      .extend(proj_ty);
  }

  // TODO: what the hell should we do with binders ...
  pub fn wrap_binder<T, O, C: FnOnce(&T) -> O>(
    value: &ty::Binder<'tcx, T>,
    f: C,
  ) -> O
  where
    T: ty::TypeFoldable<ty::TyCtxt<'tcx>>,
  {
    // let old_region_index = self.region_index;
    // let (new_value, _) = self.name_all_regions(value)?;
    let new_value = value.clone().skip_binder();
    let res = f(&new_value);
    // self.region_index = old_region_index;
    // self.binder_depth -= 1;
    res
  }
}

impl<'tcx> OpaqueImpl<'tcx> {
  pub fn new(
    def_id: DefId,
    args: &'tcx ty::List<ty::GenericArg<'tcx>>,
  ) -> Self {
    let infcx = get_dynamic_ctx();
    let tcx = infcx.tcx;

    // Grab the "TraitA + TraitB" from `impl TraitA + TraitB`,
    // by looking up the projections associated with the def_id.
    let bounds = tcx.explicit_item_bounds(def_id);

    log::debug!("Explicit item bounds {:?}", bounds);

    let mut traits = FxIndexMap::default();
    let mut fn_traits = FxIndexMap::default();
    let mut has_sized_bound = false;
    let mut has_negative_sized_bound = false;
    let mut lifetimes = SmallVec::<[ty::Region<'tcx>; 1]>::new();

    for (predicate, _) in bounds.iter_instantiated_copied(tcx, args) {
      let bound_predicate = predicate.kind();

      match bound_predicate.skip_binder() {
        ty::ClauseKind::Trait(pred) => {
          let trait_ref = bound_predicate.rebind(pred.trait_ref);

          // Don't print `+ Sized`, but rather `+ ?Sized` if absent.
          if Some(trait_ref.def_id()) == tcx.lang_items().sized_trait() {
            match pred.polarity {
              ty::ImplPolarity::Positive | ty::ImplPolarity::Reservation => {
                has_sized_bound = true;
                continue;
              }
              ty::ImplPolarity::Negative => has_negative_sized_bound = true,
            }
          }

          Self::insert_trait_and_projection(
            tcx,
            trait_ref,
            pred.polarity,
            None,
            &mut traits,
            &mut fn_traits,
          );
        }
        ty::ClauseKind::Projection(pred) => {
          let proj_ref = bound_predicate.rebind(pred);
          let trait_ref = proj_ref.required_poly_trait_ref(tcx);

          // Projection type entry -- the def-id for naming, and the ty.
          let proj_ty = (proj_ref.projection_def_id(), proj_ref.term());

          Self::insert_trait_and_projection(
            tcx,
            trait_ref,
            ty::ImplPolarity::Positive,
            Some(proj_ty),
            &mut traits,
            &mut fn_traits,
          );
        }
        ty::ClauseKind::TypeOutlives(outlives) => {
          lifetimes.push(outlives.1);
        }
        _ => {}
      }
    }

    let mut here_opaque_type = OpaqueImpl {
      fn_traits: vec![],
      traits: vec![],
      lifetimes: vec![],
      has_sized_bound: false,
      has_negative_sized_bound: false,
    };

    for (fn_once_trait_ref, entry) in fn_traits {
      Self::wrap_binder(&fn_once_trait_ref, |trait_ref| {
        let generics = tcx.generics_of(trait_ref.def_id);
        let own_args = generics.own_args_no_defaults(tcx, trait_ref.args);

        match (entry.return_ty, own_args[0].expect_ty()) {
          (Some(return_ty), arg_tys)
            if matches!(arg_tys.kind(), ty::Tuple(_)) =>
          {
            let kind = if entry.fn_trait_ref.is_some() {
              FnTraitKind::Fn
            } else if entry.fn_mut_trait_ref.is_some() {
              FnTraitKind::FnMut
            } else {
              FnTraitKind::FnOnce
            };

            let params = arg_tys.tuple_fields().iter().collect::<Vec<_>>();
            let ret_ty = return_ty.skip_binder().ty();

            here_opaque_type.fn_traits.push(FnTrait {
              params,
              ret_ty,
              kind,
            });
          }
          // If we got here, we can't print as a `impl Fn(A, B) -> C`. Just record the
          // trait_refs we collected in the OpaqueFnEntry as normal trait refs.
          _ => {
            if entry.has_fn_once {
              traits
                .entry((fn_once_trait_ref, ty::ImplPolarity::Positive))
                .or_default()
                .extend(
                  // Group the return ty with its def id, if we had one.
                  entry.return_ty.map(|ty| {
                    (tcx.require_lang_item(LangItem::FnOnceOutput, None), ty)
                  }),
                );
            }
            if let Some(trait_ref) = entry.fn_mut_trait_ref {
              traits
                .entry((trait_ref, ty::ImplPolarity::Positive))
                .or_default();
            }
            if let Some(trait_ref) = entry.fn_trait_ref {
              traits
                .entry((trait_ref, ty::ImplPolarity::Positive))
                .or_default();
            }
          }
        }
      })
    }

    // Print the rest of the trait types (that aren't Fn* family of traits)
    for ((trait_ref, polarity), assoc_items) in traits {
      Self::wrap_binder(&trait_ref, |trait_ref| {
        let trait_name = TraitRefPrintOnlyTraitPathDef::new(trait_ref);

        let generics = tcx.generics_of(trait_ref.def_id);
        let own_args = generics.own_args_no_defaults(tcx, trait_ref.args);
        let mut assoc_args = vec![];

        for (assoc_item_def_id, term) in assoc_items {
          // Skip printing `<{coroutine@} as Coroutine<_>>::Return` from async blocks,
          // unless we can find out what coroutine return type it comes from.
          let term = if let Some(ty) = term.skip_binder().ty()
            && let ty::Alias(ty::Projection, proj) = ty.kind()
            && let Some(assoc) = tcx.opt_associated_item(proj.def_id)
            && assoc.trait_container(tcx) == tcx.lang_items().coroutine_trait()
            && assoc.name == rustc_span::sym::Return
          {
            if let ty::Coroutine(_, args) = args.type_at(0).kind() {
              let return_ty = args.as_coroutine().return_ty();
              if !return_ty.is_ty_var() {
                return_ty.into()
              } else {
                continue;
              }
            } else {
              continue;
            }
          } else {
            term.skip_binder()
          };

          let name = tcx.associated_item(assoc_item_def_id).name;
          assoc_args.push(AssocItemDef { name, term });
        }

        here_opaque_type.traits.push(Trait {
          polarity,
          trait_name,
          own_args,
          assoc_args,
        });
      });
    }

    here_opaque_type.has_sized_bound = has_sized_bound;
    here_opaque_type.has_negative_sized_bound = has_negative_sized_bound;

    here_opaque_type
  }
}
