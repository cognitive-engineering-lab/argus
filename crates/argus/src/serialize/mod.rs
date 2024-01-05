//! Remote serde::Serialize derives for Rustc types
//!
//! WARNING, these definitions were done hastily, and definitely
//! need a little "fixing up." It will be done at some point.
//! In the meantime, consume at your own risk.
#![allow(
    non_camel_case_types,
    non_snake_case,
    suspicious_double_ref_op,
    dead_code
)]

pub mod r#const;
pub mod path;
pub mod term;
pub mod ty;

use r#const::*;
use path::*;
use term::*;
use ty::*;

use std::num::*;

use rustc_type_ir as ir;
use rustc_infer::infer::{InferCtxt, type_variable::TypeVariableOriginKind};
use rustc_middle::{ty::{*, abstract_const::CastKind}, mir::{BinOp, UnOp}};
use rustc_hir::def_id::{DefId, DefIndex, CrateNum};
use rustc_span::symbol::Symbol;
use rustc_target::spec::abi::Abi;
use rustc_hir::Unsafety;
use rustc_trait_selection::traits::solve::Goal;

use serde::{Serialize, ser::SerializeSeq};

/// Entry function to serialize anything from rustc.
pub fn serialize_to_value<'tcx, T: Serialize + 'tcx>(
    value: &T, infcx: &InferCtxt<'tcx>
) -> Result<serde_json::Value, serde_json::Error> {
    in_dynamic_ctx(infcx, || serde_json::to_value(&value))
}

// NOTE: setting the dynamic TCX should *only* happen
// before calling the serialize function, it must guarantee
// that the 'tcx lifetime is the same as that of the serialized item.
fluid_let::fluid_let!{static INFCX: &'static InferCtxt<'static>}

fn in_dynamic_ctx<'tcx, T>(infcx: &InferCtxt<'tcx>, f: impl FnOnce() -> T) -> T {
    let infcx: &'static InferCtxt<'static> = unsafe { std::mem::transmute(infcx) };
    INFCX.set(infcx, f)
}

fn get_dynamic_ctx<'a, 'tcx: 'a>() -> &'a InferCtxt<'tcx> {
    let infcx: &'static InferCtxt<'static> = INFCX.copied().unwrap();
    unsafe { std::mem::transmute::<
            &'static InferCtxt<'static>,
            &'a InferCtxt<'tcx>
        >(infcx) }
}

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

pub(crate) use serialize_custom_seq;
