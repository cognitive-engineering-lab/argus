use std::marker::PhantomData;

use rustc_abi::Size;
use rustc_apfloat::{
  ieee::{Double, Single},
  Float,
};
use rustc_hir::def::DefKind;
use rustc_middle::ty::*;
use rustc_span::Symbol;
use serde::Serialize;
#[cfg(feature = "testing")]
use ts_rs::TS;

use super::{
  term::*,
  ty::{BoundVariable, SymbolDef},
  *,
};

// TODO: one thing missing is being able to print
// the `Ty` after the constant syntactic definition.
#[derive(Many)]
#[argus(remote = "Const")]
pub struct ConstDef<'tcx>(PhantomData<&'tcx ()>);
impl<'tcx> ConstDef<'tcx> {
  pub fn serialize<S>(value: &Const<'tcx>, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    ConstKindDef::from(value).serialize(s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "Const"))]
#[serde(tag = "type")]
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
    #[cfg_attr(feature = "testing", ts(type = "BoundVariable"))]
    data: BoundVariable,
  },
  Placeholder,
  Value {
    #[serde(with = "ValueDef")]
    #[cfg_attr(feature = "testing", ts(type = "Value"))]
    data: Value<'tcx>,
  },
  Error,
  Expr {
    data: ExprDef<'tcx>,
  },
}

impl<'tcx> From<&Const<'tcx>> for ConstKindDef<'tcx> {
  fn from(value: &Const<'tcx>) -> Self {
    let kind = value.kind();

    match kind {
      ConstKind::Unevaluated(uc) => Self::Unevaluated { data: uc },
      ConstKind::Param(v) => Self::Param { data: v },
      ConstKind::Value(data) => Self::Value { data },
      ConstKind::Expr(e) => Self::Expr {
        data: ExprDef::from(&e),
      },
      ConstKind::Error(..) => Self::Error,

      ConstKind::Bound(didx, bv) => Self::Bound {
        data: BoundVariable::new(didx, bv),
      },
      ConstKind::Infer(ic) => Self::Infer { data: ic },
      ConstKind::Placeholder(..) => Self::Placeholder,
    }
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "InferConst"))]
pub enum InferConstDef {
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

impl InferConstDef {
  pub fn new(value: &InferConst) -> Self {
    // TODO: can we get the name of an inference variable?
    match value {
      InferConst::Fresh(_) | InferConst::Var(_) => Self::Anon,
    }
  }

  pub fn serialize<S>(value: &InferConst, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self::new(value).serialize(s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "ParamConst"))]
pub struct ParamConstDef(
  #[serde(with = "SymbolDef")]
  #[cfg_attr(feature = "testing", ts(type = "Symbol"))]
  Symbol,
);

impl ParamConstDef {
  pub fn serialize<S>(value: &ParamConst, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self(value.name).serialize(s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "UnevaluatedConst"))]
#[serde(tag = "type")]
enum UnevaluatedConstDef<'tcx> {
  ValuePath {
    data: path::ValuePathWithArgs<'tcx>,
  },
  AnonSnippet {
    data: String,
  },
  AnonLocation {
    #[serde(with = "SymbolDef")]
    #[cfg_attr(feature = "testing", ts(type = "Symbol"))]
    krate: Symbol,

    path: path::BasicPathNoArgs<'tcx>,
  },
}

impl<'tcx> UnevaluatedConstDef<'tcx> {
  fn new(value: &UnevaluatedConst<'tcx>) -> Self {
    InferCtxt::access(|infcx| {
      let UnevaluatedConst { def, args } = value;
      match infcx.tcx.def_kind(def) {
        DefKind::Const | DefKind::AssocConst => Self::ValuePath {
          data: path::ValuePathWithArgs::new(*def, args),
        },
        DefKind::AnonConst => {
          if def.is_local() {
            let span = infcx.tcx.def_span(def);
            if let Ok(snip) = infcx.tcx.sess.source_map().span_to_snippet(span)
            {
              return Self::AnonSnippet { data: snip };
            }
          }

          // Do not call `print_value_path` as if a parent of this anon const is an impl it will
          // attempt to print out the impl trait ref i.e. `<T as Trait>::{constant#0}`. This would
          // cause printing to enter an infinite recursion if the anon const is in the self type i.e.
          // `impl<T: Default> Default for [T; 32 - 1 - 1 - 1] {`
          // where we would try to print `<[T; /* print `constant#0` again */] as Default>::{constant#0}`
          Self::AnonLocation {
            krate: infcx.tcx.crate_name(def.krate),
            path: path::BasicPathNoArgs::new(*def),
          }
        }
        defkind => panic!("unexpected defkind {defkind:?} {value:?}"),
      }
    })
  }

  pub fn serialize<S>(
    value: &UnevaluatedConst<'tcx>,
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
#[cfg_attr(feature = "testing", ts(export, rename = "ConstScalarInt"))]
#[serde(tag = "type")]
pub enum ConstScalarIntDef {
  False,
  True,
  #[serde(rename_all = "camelCase")]
  Float {
    data: String,
    is_finite: bool,
  },
  Int {
    data: String,
  },
  Char {
    data: String,
  },
  Misc {
    data: String,
  },
}

impl ConstScalarIntDef {
  pub fn new(int: ScalarInt, ty: Ty) -> Self {
    InferCtxt::access(|infcx| {
      let tcx = infcx.tcx;
      match ty.kind() {
        Bool if int == ScalarInt::FALSE => Self::False,
        Bool if int == ScalarInt::TRUE => Self::True,
        Float(FloatTy::F32) => {
          let val = Single::from(int);
          Self::Float {
            data: format!("{val}"),
            is_finite: val.is_finite(),
          }
        }
        Float(FloatTy::F64) => {
          let val = Double::from(int);
          Self::Float {
            data: format!("{val}"),
            is_finite: val.is_finite(),
          }
        }
        Uint(_) | Int(_) => {
          let int = ConstInt::new(
            int,
            matches!(ty.kind(), Int(_)),
            ty.is_ptr_sized_integral(),
          );
          Self::Int {
            data: format!("{int:?}"),
          }
        }
        Char if char::try_from(int).is_ok() => Self::Char {
          data: format!("{}", char::try_from(int).is_ok()),
        },
        Ref(..) | RawPtr(..) | FnPtr(..) => {
          let data = int.to_bits(tcx.data_layout.pointer_size);
          Self::Misc {
            data: format!("0x{data:x}"),
          }
        }
        _ => {
          if int.size() == Size::ZERO {
            Self::Misc {
              data: "transmute(())".to_string(),
            }
          } else {
            Self::Misc {
              data: format!("transmute(0x{int:x})"),
            }
          }
        }
      }
    })
  }
}
