use rustc_hir::def::CtorKind;
use rustc_middle::{
  mir::{BinOp, UnOp},
  ty::{self, abstract_const::CastKind, *},
};
use rustc_span::Symbol;
use serde::Serialize;

use super::*;

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

pub struct ValTreeDef<'a, 'tcx: 'a> {
  tree: &'a ValTree<'tcx>,
  ty: &'a Ty<'tcx>,
}

impl<'a, 'tcx: 'a> ValTreeDef<'a, 'tcx> {
  pub fn new(tree: &'a ValTree<'tcx>, ty: &'a Ty<'tcx>) -> Self {
    Self { tree, ty }
  }
}

// See rustc_middle::ty::print::pretty::pretty_print_const_valtree
impl<'a, 'tcx: 'a> Serialize for ValTreeDef<'a, 'tcx> {
  fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let infcx = get_dynamic_ctx();
    let tcx = &infcx.tcx;

    #[derive(Serialize)]
    enum ValTreeKind<'a, 'tcx: 'a> {
      Ref {
        inner: ValTreeDef<'a, 'tcx>,
      },

      String {
        data: String,
        is_deref: bool,
      },

      Aggregate {
        #[serde(with = "Slice__ConstDef")]
        fields: &'tcx [Const<'tcx>],
        kind: ValTreeAggregateKind<'tcx>,
      },

      Leaf {
        data: ConstScalarIntDef<'tcx>,
        kind: LeafKind,
      },
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase", tag = "type")]
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
    #[serde(rename_all = "camelCase", tag = "type")]
    enum AdtAggregateKind {
      Fn,
      Const,
      Misc {
        #[serde(with = "Slice__SymbolDef")]
        names: Vec<Symbol>,
      },
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase", tag = "type")]
    enum LeafKind {
      Ref,
      Scalar,
    }

    let u8_type = tcx.types.u8;
    let here_kind = match (self.tree, self.ty.kind()) {
      (ty::ValTree::Branch(_), ty::Ref(_, inner_ty, _)) => {
        match inner_ty.kind() {
          ty::Slice(t) if *t == u8_type => {
            let bytes = self
              .tree
              .try_to_raw_bytes(*tcx, *self.ty)
              .unwrap_or_else(|| {
                panic!("expected bytes from slice valtree");
              });
            ValTreeKind::String {
              data: format!("b\"{}\"", bytes.escape_ascii()),
              is_deref: false,
            }
          }
          ty::Str => {
            let bytes = self
              .tree
              .try_to_raw_bytes(*tcx, *self.ty)
              .unwrap_or_else(|| {
                panic!("expected bytes from slice valtree");
              });
            ValTreeKind::String {
              data: String::from_utf8_lossy(bytes).to_string(),
              is_deref: false,
            }
          }
          _ => ValTreeKind::Ref {
            inner: ValTreeDef::new(self.tree, inner_ty),
          },
        }
      }
      (ty::ValTree::Branch(_), ty::Array(t, _)) if *t == u8_type => {
        let bytes =
          self
            .tree
            .try_to_raw_bytes(*tcx, *self.ty)
            .unwrap_or_else(|| {
              panic!("expected bytes from slice valtree");
            });

        ValTreeKind::String {
          data: format!("b\"{}\"", bytes.escape_ascii()),
          is_deref: true,
        }
      }

      (ty::ValTree::Branch(_), ty::Array(..) | ty::Tuple(..) | ty::Adt(..)) => {
        let contents = tcx
          .destructure_const(ty::Const::new_value(*tcx, *self.tree, *self.ty));
        let fields = contents.fields;
        let kind = match self.ty.kind() {
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
        data: ConstScalarIntDef::new(*leaf, *inner_ty),
        kind: LeafKind::Ref,
      },
      (ty::ValTree::Leaf(leaf), _) => ValTreeKind::Leaf {
        data: ConstScalarIntDef::new(*leaf, *self.ty),
        kind: LeafKind::Scalar,
      },
      _ => ValTreeKind::String {
        // TODO: I don't fully understand this fallback case, revisit it later!
        data: "VALTREE".to_string(),
        is_deref: false,
      },
    };

    here_kind.serialize(s)
  }
}

#[derive(Serialize)]
#[serde(remote = "Expr")]
pub enum ExprDef<'tcx> {
  Binop(
    #[serde(with = "BinOpDef")] BinOp,
    #[serde(with = "ConstDef")] Const<'tcx>,
    #[serde(with = "ConstDef")] Const<'tcx>,
  ),
  UnOp(
    #[serde(with = "UnOpDef")] UnOp,
    #[serde(with = "ConstDef")] Const<'tcx>,
  ),
  FunctionCall(
    #[serde(with = "ConstDef")] Const<'tcx>,
    #[serde(serialize_with = "list__const")] &'tcx List<Const<'tcx>>,
  ),
  Cast(
    #[serde(with = "CastKindDef")] CastKind,
    #[serde(with = "ConstDef")] Const<'tcx>,
    #[serde(with = "TyDef")] Ty<'tcx>,
  ),
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
