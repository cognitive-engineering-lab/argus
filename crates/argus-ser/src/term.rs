use rustc_hir::def::CtorKind;
use rustc_middle::{
  mir::{BinOp, UnOp},
  ty::{self, abstract_const::CastKind, *},
};
use rustc_span::Symbol;
use serde::Serialize;
#[cfg(feature = "testing")]
use ts_rs::TS;

use super::{r#const::*, ty::*, *};

pub struct TermDef;
impl TermDef {
  pub fn serialize<S>(value: &Term, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    TermKindDef::serialize(&value.unpack(), s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "Term"))]
#[serde(remote = "TermKind")]
pub enum TermKindDef<'tcx> {
  Ty(
    #[serde(with = "TyDef")]
    #[cfg_attr(feature = "testing", ts(type = "Ty"))]
    Ty<'tcx>,
  ),
  Const(
    #[serde(with = "ConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "Const"))]
    Const<'tcx>,
  ),
}

#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "Value"))]
pub struct ValueDef<'tcx> {
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  pub ty: Ty<'tcx>,
  #[cfg_attr(feature = "testing", ts(type = "ValTree"))]
  pub valtree: ValTree<'tcx>,
}

impl ValueDef<'_> {
  pub fn serialize<S>(value: &Value, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    ValTreeKind::from(value).serialize(s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "ValTree"))]
#[serde(tag = "type")]
enum ValTreeKind<'tcx> {
  Ref {
    #[serde(with = "ValueDef")]
    #[cfg_attr(feature = "testing", ts(type = "Value"))]
    inner: Value<'tcx>,
  },

  #[serde(rename_all = "camelCase")]
  String { data: String, is_deref: bool },

  Aggregate {
    #[serde(with = "ConstDefs")]
    #[cfg_attr(feature = "testing", ts(type = "Const[]"))]
    fields: &'tcx [Const<'tcx>],

    kind: ValTreeAggregateKind<'tcx>,
  },

  Leaf {
    #[cfg_attr(feature = "testing", ts(type = "ConstScalarInt"))]
    data: ConstScalarIntDef,
    kind: LeafKind,
  },
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
#[serde(tag = "type")]
enum ValTreeAggregateKind<'tcx> {
  Array,
  Tuple,
  AdtNoVariants,
  Adt {
    data: path::ValuePathWithArgs<'tcx>,
    kind: AdtAggregateKind,
  },
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
#[serde(tag = "type")]
enum AdtAggregateKind {
  Fn,
  Const,
  Misc {
    #[serde(with = "SymbolDefs")]
    #[cfg_attr(feature = "testing", ts(type = "Symbol[]"))]
    names: Vec<Symbol>,
  },
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
#[serde(tag = "type")]
enum LeafKind {
  Ref,
  Scalar,
}

impl<'tcx> From<&Value<'tcx>> for ValTreeKind<'tcx> {
  fn from(value: &Value<'tcx>) -> Self {
    InferCtxt::access(|infcx| {
      let tcx = &infcx.tcx;
      let this = value;

      let u8_type = tcx.types.u8;
      match (*this.valtree, this.ty.kind()) {
        (ty::ValTreeKind::Branch(_), ty::Ref(_, inner_ty, _)) => {
          match inner_ty.kind() {
            ty::Slice(t) if *t == u8_type => {
              let bytes = this.try_to_raw_bytes(*tcx).unwrap_or_else(|| {
                panic!("expected bytes from slice valtree");
              });
              ValTreeKind::String {
                data: format!("b\"{}\"", bytes.escape_ascii()),
                is_deref: false,
              }
            }
            ty::Str => {
              let bytes = this.try_to_raw_bytes(*tcx).unwrap_or_else(|| {
                panic!("expected bytes from slice valtree");
              });
              ValTreeKind::String {
                data: String::from_utf8_lossy(bytes).to_string(),
                is_deref: false,
              }
            }
            _ => ValTreeKind::Ref { inner: *this },
          }
        }
        (ty::ValTreeKind::Branch(_), ty::Array(t, _)) if *t == u8_type => {
          let bytes = this.try_to_raw_bytes(*tcx).unwrap_or_else(|| {
            panic!("expected bytes from slice valtree");
          });

          ValTreeKind::String {
            data: format!("b\"{}\"", bytes.escape_ascii()),
            is_deref: true,
          }
        }

        (
          ty::ValTreeKind::Branch(_),
          ty::Array(..) | ty::Tuple(..) | ty::Adt(..),
        ) => {
          let contents = tcx.destructure_const(ty::Const::new_value(
            *tcx,
            this.valtree,
            this.ty,
          ));
          let fields = contents.fields;
          let kind = match this.ty.kind() {
            ty::Array(..) => ValTreeAggregateKind::Array,
            ty::Tuple(..) => ValTreeAggregateKind::Tuple,
            ty::Adt(def, _) if def.variants().is_empty() => {
              ValTreeAggregateKind::AdtNoVariants
            }
            ty::Adt(def, args) => {
              let variant_idx = contents
                .variant
                .expect("destructed const of adt without variant idx");
              let variant_def = &def.variant(variant_idx);
              let value_path =
                path::ValuePathWithArgs::new(variant_def.def_id, args);
              let adt_kind = match variant_def.ctor_kind() {
                Some(CtorKind::Const) => AdtAggregateKind::Const,
                Some(CtorKind::Fn) => AdtAggregateKind::Fn,
                _ => AdtAggregateKind::Misc {
                  names: variant_def
                    .fields
                    .iter()
                    .map(|field_def| field_def.name)
                    .collect::<Vec<_>>(),
                },
              };

              ValTreeAggregateKind::Adt {
                data: value_path,
                kind: adt_kind,
              }
            }
            _ => unreachable!(),
          };

          ValTreeKind::Aggregate { fields, kind }
        }

        (ty::ValTreeKind::Leaf(leaf), ty::Ref(_, inner_ty, _)) => {
          ValTreeKind::Leaf {
            data: ConstScalarIntDef::new(*leaf, *inner_ty),
            kind: LeafKind::Ref,
          }
        }
        (ty::ValTreeKind::Leaf(leaf), _) => ValTreeKind::Leaf {
          data: ConstScalarIntDef::new(*leaf, this.ty),
          kind: LeafKind::Scalar,
        },
        _ => ValTreeKind::String {
          // TODO: I don't fully understand this fallback case, revisit it later!
          data: "VALTREE".to_string(),
          is_deref: false,
        },
      }
    })
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum ExprDef<'tcx> {
  Binop(
    #[serde(with = "BinOpDef")]
    #[cfg_attr(feature = "testing", ts(type = "BinOp"))]
    BinOp,
    #[serde(with = "ConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "Const"))]
    Const<'tcx>,
    #[serde(with = "ConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "Const"))]
    Const<'tcx>,
  ),
  UnOp(
    #[serde(with = "UnOpDef")]
    #[cfg_attr(feature = "testing", ts(type = "UnOp"))]
    UnOp,
    #[serde(with = "ConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "Const"))]
    Const<'tcx>,
  ),
  FunctionCall(
    #[serde(with = "ConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "Const"))]
    Const<'tcx>,
    #[serde(with = "ConstDefs")]
    #[cfg_attr(feature = "testing", ts(type = "Const[]"))]
    Vec<Const<'tcx>>,
  ),
  Cast(
    #[serde(with = "CastKindDef")]
    #[cfg_attr(feature = "testing", ts(type = "CastKind"))]
    CastKind,
    #[serde(with = "ConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "Const"))]
    Const<'tcx>,
    #[serde(with = "TyDef")]
    #[cfg_attr(feature = "testing", ts(type = "Ty"))]
    Ty<'tcx>,
  ),
}

impl<'tcx> From<&Expr<'tcx>> for ExprDef<'tcx> {
  fn from(value: &Expr<'tcx>) -> Self {
    use rustc_middle::ty::ExprKind::*;
    match value.kind {
      Binop(op) => {
        let (_t1, _ty2, lhs, rhs) = value.binop_args();
        Self::Binop(op, lhs, rhs)
      }
      UnOp(op) => {
        let (_t1, val) = value.unop_args();
        Self::UnOp(op, val)
      }
      FunctionCall => {
        let (_ty, val, args) = value.call_args();
        Self::FunctionCall(val, args.collect())
      }
      Cast(kind) => {
        let (_t1, val, ty) = value.cast_args();
        Self::Cast(kind, val, ty)
      }
    }
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "BinOp"))]
#[serde(remote = "BinOp")]
pub enum BinOpDef {
  Add,
  AddUnchecked,
  AddWithOverflow,
  Cmp,
  Sub,
  SubUnchecked,
  SubWithOverflow,
  Mul,
  MulUnchecked,
  MulWithOverflow,
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
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "UnOp"))]
#[serde(remote = "UnOp")]
pub enum UnOpDef {
  Not,
  Neg,
  PtrMetadata,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "CastKind"))]
#[serde(remote = "CastKind")]
pub enum CastKindDef {
  As,
  Use,
}
