use rustc_apfloat::{
  ieee::{Double, Single},
  Float,
};
use rustc_hir::def::DefKind;
use rustc_middle::ty::*;
use rustc_span::Symbol;
use rustc_target::abi::Size;
use serde::Serialize;
#[cfg(feature = "testing")]
use ts_rs::TS;

use super::*;

// TODO: one thing missing is being able to print
// the `Ty` after the constant syntactic definition.
pub struct ConstDef;
impl ConstDef {
  pub fn serialize<'tcx, S>(
    value: &Const<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    ConstKindDef::from(value).serialize(s)
  }
}

pub struct Slice__ConstDef;
impl Slice__ConstDef {
  pub fn serialize<'tcx, S>(
    value: &[Const<'tcx>],
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    struct Wrapper<'a, 'tcx>(#[serde(with = "ConstDef")] &'a Const<'tcx>);
    serialize_custom_seq! { Wrapper, s, value }
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "Const"))]
#[serde(tag = "type", rename_all = "camelCase")]
enum ConstKindDef<'tcx> {
  Unevaluated {
    #[serde(with = "UnevaluatedConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "UnevaluatedConst"))]
    data: UnevaluatedConst<'tcx>,
  },
  Param {
    #[serde(with = "ParamConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "ParamConst"))]
    data: ParamConst,
  },
  Infer {
    #[serde(with = "InferConstDef")]
    #[cfg_attr(feature = "testing", ts(type = "InferConst"))]
    data: InferConst,
  },
  Bound {
    #[cfg_attr(feature = "testing", ts(type = "any"))]
    data: BoundVariable,
  },
  // TODO:
  // Placeholder {
  //   #[serde(skip)] // TODO:
  //   data: &'a Placeholder<BoundVar>,
  // },
  Value {
    #[cfg_attr(feature = "testing", ts(type = "ValTree"))]
    data: ValTreeDef<'tcx>,
  },
  Error,
  Expr {
    #[serde(with = "ExprDef")]
    #[cfg_attr(feature = "testing", ts(type = "Expr"))]
    data: Expr<'tcx>,
  },
}

impl<'a, 'tcx: 'a> From<&Const<'tcx>> for ConstKindDef<'tcx> {
  fn from(value: &Const<'tcx>) -> Self {
    let self_ty = value.ty();
    let kind = value.kind();

    match kind {
      ConstKind::Unevaluated(uc) => ConstKindDef::Unevaluated { data: uc },
      ConstKind::Param(v) => ConstKindDef::Param { data: v },
      ConstKind::Value(v) => ConstKindDef::Value {
        data: ValTreeDef::new(v, self_ty),
      },
      ConstKind::Expr(e) => ConstKindDef::Expr { data: e },
      ConstKind::Error(..) => ConstKindDef::Error,

      ConstKind::Bound(didx, bv) => ConstKindDef::Bound {
        data: BoundVariable::new(didx, bv),
      },
      ConstKind::Infer(ic) => ConstKindDef::Infer { data: ic },
      ConstKind::Placeholder(..) => todo!(),
    }
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "InferConst"))]
#[serde(rename_all = "camelCase", tag = "type")]
enum InferConstKindDef {
  // TODO: the `ConstVariableOrigin` doesn't seem to be publicly exposed.
  // If it were, we could probe the InferCtxt for the origin of an unresolved
  // infer var, potentially resulting in a named constant. But that isn't possible
  // yet. (At least it doesn't seem to be.)
  // Named {
  //   #[serde(with = "SymbolDef")]
  //   data: Symbol,
  // },
  Anon,
}

pub struct InferConstDef;
impl InferConstDef {
  pub fn serialize<'tcx, S>(value: &InferConst, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    InferConstKindDef::from(value).serialize(s)
  }
}

impl From<&InferConst> for InferConstKindDef {
  fn from(value: &InferConst) -> Self {
    // TODO: can we get the name of an inference variable?
    match value {
      InferConst::Fresh(_) | InferConst::EffectVar(_) | InferConst::Var(_) => {
        InferConstKindDef::Anon
      }
    }
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "ParamConst"))]
pub struct ParamConstDefDef(
  #[serde(with = "SymbolDef")]
  #[cfg_attr(feature = "testing", ts(type = "Symbol"))]
  Symbol,
);

pub struct ParamConstDef;
impl ParamConstDef {
  pub fn serialize<'tcx, S>(value: &ParamConst, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    ParamConstDefDef(value.name).serialize(s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "UnevaluatedConst"))]
#[serde(rename_all = "camelCase", tag = "type")]
enum UnevaluatedConstKind<'tcx> {
  ValuePath {
    #[cfg_attr(feature = "testing", ts(type = "any"))]
    data: path::ValuePathWithArgs<'tcx>,
  },
  AnonSnippet {
    data: String,
  },
}

impl<'tcx> From<&UnevaluatedConst<'tcx>> for UnevaluatedConstKind<'tcx> {
  fn from(value: &UnevaluatedConst<'tcx>) -> Self {
    let infcx = get_dynamic_ctx();
    let UnevaluatedConst { def, args } = value;
    match infcx.tcx.def_kind(def) {
      DefKind::Const | DefKind::AssocConst => UnevaluatedConstKind::ValuePath {
        data: path::ValuePathWithArgs::new(*def, args),
      },
      DefKind::AnonConst => {
        if def.is_local()
          && let span = infcx.tcx.def_span(def)
          && let Ok(snip) = infcx.tcx.sess.source_map().span_to_snippet(span)
        {
          UnevaluatedConstKind::AnonSnippet { data: snip }
        } else {
          todo!()
        }
      }
      defkind => panic!("unexpected defkind {:?} {:?}", defkind, value),
    }
  }
}

pub struct UnevaluatedConstDef;
impl UnevaluatedConstDef {
  pub fn serialize<'tcx, S>(
    value: &UnevaluatedConst<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    UnevaluatedConstKind::from(value).serialize(s)
  }
}

pub struct AliasConstDef;
impl AliasConstDef {
  pub fn serialize<'tcx, S>(
    value: &UnevaluatedConst<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    UnevaluatedConstDef::serialize(value, s)
  }
}

pub struct BoundConstDef;
impl BoundConstDef {
  pub fn serialize<'tcx, S>(value: &BoundVar, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    // BoundVarDef::serialize(value, s)
    todo!()
  }
}

pub struct ExprConstDef;
impl ExprConstDef {
  pub fn serialize<'tcx, S>(value: &Expr<'tcx>, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    ExprDef::serialize(value, s)
  }
}

pub struct ConstScalarIntDef<'tcx> {
  int: ScalarInt,
  ty: Ty<'tcx>,
}

impl<'tcx> ConstScalarIntDef<'tcx> {
  pub fn new(int: ScalarInt, ty: Ty<'tcx>) -> Self {
    Self { int, ty }
  }
}

impl<'tcx> Serialize for ConstScalarIntDef<'tcx> {
  fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase", tag = "type")]
    enum ConstScalarIntKind {
      False,
      True,
      Float { data: String, is_finite: bool },
      Int { data: String },
      Char { data: String },
      Misc { data: String },
    }

    let infcx = get_dynamic_ctx();
    let tcx = infcx.tcx;

    let here_kind = match self.ty.kind() {
      Bool if self.int == ScalarInt::FALSE => ConstScalarIntKind::False,
      Bool if self.int == ScalarInt::TRUE => ConstScalarIntKind::True,
      Float(FloatTy::F32) => {
        let val = Single::try_from(self.int).unwrap();
        ConstScalarIntKind::Float {
          data: format!("{val}"),
          is_finite: val.is_finite(),
        }
      }
      Float(FloatTy::F64) => {
        let val = Double::try_from(self.int).unwrap();
        ConstScalarIntKind::Float {
          data: format!("{val}"),
          is_finite: val.is_finite(),
        }
      }
      Uint(_) | Int(_) => {
        let int = ConstInt::new(
          self.int,
          matches!(self.ty.kind(), Int(_)),
          self.ty.is_ptr_sized_integral(),
        );
        ConstScalarIntKind::Int {
          data: format!("{}", self.int),
        }
      }
      Char if char::try_from(self.int).is_ok() => ConstScalarIntKind::Char {
        data: format!("{}", char::try_from(self.int).is_ok()),
      },
      Ref(..) | RawPtr(..) | FnPtr(_) => {
        let data = self.int.assert_bits(tcx.data_layout.pointer_size);
        ConstScalarIntKind::Misc {
          data: format!("0x{data:x}"),
        }
      }
      _ => {
        if self.int.size() == Size::ZERO {
          ConstScalarIntKind::Misc {
            data: "transmute(())".to_string(),
          }
        } else {
          ConstScalarIntKind::Misc {
            data: format!("transmute(0x{:x})", self.int),
          }
        }
      }
    };

    here_kind.serialize(s)
  }
}
