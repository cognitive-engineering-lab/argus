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
  pub fn serialize<'tcx, S>(value: &Term<'tcx>, s: S) -> Result<S::Ok, S::Error>
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

pub struct ValTreeDef<'tcx> {
  tree: ValTree<'tcx>,
  ty: Ty<'tcx>,
}

impl<'tcx> ValTreeDef<'tcx> {
  pub fn new(tree: ValTree<'tcx>, ty: Ty<'tcx>) -> Self {
    Self { tree, ty }
  }
}

impl<'tcx> Serialize for ValTreeDef<'tcx> {
  fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    ValTreeKind::from(self).serialize(s)
  }
}

// NOTE: inner types for a ValTreeDef

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "ValTree"))]
#[serde(tag = "type")]
enum ValTreeKind<'tcx> {
  Ref {
    #[cfg_attr(feature = "testing", ts(type = "ValTree"))]
    inner: ValTreeDef<'tcx>,
  },

  #[serde(rename_all = "camelCase")]
  String { data: String, is_deref: bool },

  Aggregate {
    #[serde(with = "Slice__ConstDef")]
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
    #[serde(with = "Slice__SymbolDef")]
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

impl<'tcx> From<&ValTreeDef<'tcx>> for ValTreeKind<'tcx> {
  fn from(value: &ValTreeDef<'tcx>) -> Self {
    let infcx = get_dynamic_ctx();
    let tcx = &infcx.tcx;
    let this = value;

    let u8_type = tcx.types.u8;
    match (this.tree, this.ty.kind()) {
      (ty::ValTree::Branch(_), ty::Ref(_, inner_ty, _)) => {
        match inner_ty.kind() {
          ty::Slice(t) if *t == u8_type => {
            let bytes = this
              .tree
              .try_to_raw_bytes(*tcx, this.ty)
              .unwrap_or_else(|| {
                panic!("expected bytes from slice valtree");
              });
            ValTreeKind::String {
              data: format!("b\"{}\"", bytes.escape_ascii()),
              is_deref: false,
            }
          }
          ty::Str => {
            let bytes = this
              .tree
              .try_to_raw_bytes(*tcx, this.ty)
              .unwrap_or_else(|| {
                panic!("expected bytes from slice valtree");
              });
            ValTreeKind::String {
              data: String::from_utf8_lossy(bytes).to_string(),
              is_deref: false,
            }
          }
          _ => ValTreeKind::Ref {
            inner: ValTreeDef::new(this.tree, *inner_ty),
          },
        }
      }
      (ty::ValTree::Branch(_), ty::Array(t, _)) if *t == u8_type => {
        let bytes =
          this
            .tree
            .try_to_raw_bytes(*tcx, this.ty)
            .unwrap_or_else(|| {
              panic!("expected bytes from slice valtree");
            });

        ValTreeKind::String {
          data: format!("b\"{}\"", bytes.escape_ascii()),
          is_deref: true,
        }
      }

      (ty::ValTree::Branch(_), ty::Array(..) | ty::Tuple(..) | ty::Adt(..)) => {
        let contents =
          tcx.destructure_const(ty::Const::new_value(*tcx, this.tree, this.ty));
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

      (ty::ValTree::Leaf(leaf), ty::Ref(_, inner_ty, _)) => ValTreeKind::Leaf {
        data: ConstScalarIntDef::new(leaf, *inner_ty),
        kind: LeafKind::Ref,
      },
      (ty::ValTree::Leaf(leaf), _) => ValTreeKind::Leaf {
        data: ConstScalarIntDef::new(leaf, this.ty),
        kind: LeafKind::Scalar,
      },
      _ => ValTreeKind::String {
        // TODO: I don't fully understand this fallback case, revisit it later!
        data: "VALTREE".to_string(),
        is_deref: false,
      },
    }
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
#[serde(remote = "Expr")]
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
    #[serde(with = "Slice__ConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "Const[]"))]
    &'tcx List<Const<'tcx>>,
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

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "BinOp"))]
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
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "UnOp"))]
#[serde(remote = "UnOp")]
pub enum UnOpDef {
  Not,
  Neg,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "CastKind"))]
#[serde(remote = "CastKind")]
pub enum CastKindDef {
  As,
  Use,
}
