use std::num::*;

use rustc_type_ir as ir;
use rustc_infer::infer::{InferCtxt, type_variable::TypeVariableOriginKind};
use rustc_middle::{ty::{self, *, abstract_const::CastKind}, mir::{BinOp, UnOp}};
use rustc_hir::def_id::{DefId, DefIndex, CrateNum};
use rustc_span::symbol::{Symbol, kw};
use rustc_target::spec::abi::Abi;
use rustc_hir::Unsafety;

use serde::{Serialize, ser::SerializeSeq};

use super::*;
use my_ty::*;

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
#[serde(remote = "ConstData")]
pub struct ConstDataDef<'tcx> {
    #[serde(with = "TyDef")]
    pub ty: Ty<'tcx>,
    #[serde(skip)]
    pub kind: ConstKind<'tcx>,
}

pub struct UnevaluatedConstDef;
impl UnevaluatedConstDef {
    pub fn serialize<'tcx, S>(value: &UnevaluatedConst<'tcx>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        path::PathDefWithArgs::new(value.def, value.args).serialize(s)
    }
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

pub struct BoundConstDef;
impl BoundConstDef {
    pub fn serialize<'tcx, S>(value: &BoundVar, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        BoundVarDef::serialize(value, s)
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

pub struct ExprConstDef;
impl ExprConstDef {
    pub fn serialize<'tcx, S>(value: &Expr<'tcx>, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        ExprDef::serialize(value, s)
    }
}

pub fn list__const<'tcx, S>(value: &List<Const<'tcx>>, s: S) -> Result<S::Ok, S::Error>
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
#[serde(remote = "PlaceholderConst")]
struct PlaceholderConstDef {
    #[serde(with = "UniverseIndexDef")]
    pub universe: UniverseIndex,
    #[serde(with = "BoundVarDef")]
    pub bound: BoundVar,
}
