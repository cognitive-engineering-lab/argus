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
use r#const::*;
use my_ty::*;

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
#[serde(remote = "ValTree")]
pub enum ValTreeDef<'tcx> {
    Leaf(#[serde(with = "ScalarIntDef")] ScalarInt),
    Branch(#[serde(serialize_with = "slice__val_tree")] &'tcx [ValTree<'tcx>]),
}

#[derive(Serialize)]
#[serde(remote = "Expr")]
pub enum ExprDef<'tcx> {
    Binop(#[serde(with = "BinOpDef")] BinOp, #[serde(with = "ConstDef")] Const<'tcx>, #[serde(with = "ConstDef")] Const<'tcx>),
    UnOp(#[serde(with = "UnOpDef")] UnOp, #[serde(with = "ConstDef")] Const<'tcx>),
    FunctionCall(#[serde(with = "ConstDef")] Const<'tcx>, #[serde(serialize_with = "list__const")] &'tcx List<Const<'tcx>>),
    Cast(#[serde(with = "CastKindDef")] CastKind, #[serde(with = "ConstDef")] Const<'tcx>, #[serde(with = "TyDef")] Ty<'tcx>),
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
