use rustc_type_ir as ir;
use rustc_middle::{ty::{self, *, abstract_const::CastKind}, mir::{BinOp, UnOp}};
use rustc_hir::def_id::{DefId, DefIndex, CrateNum};
use rustc_data_structures::sso::SsoHashSet;
use rustc_span::{symbol::{kw, sym, Ident, Symbol}, Span};
use rustc_target::spec::abi::Abi;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_hir::Unsafety;
use rustc_session::cstore::ExternCrateSource;
// use rustc_middle::ty::print::with_crate_prefix;
use rustc_session::cstore::ExternCrate;
use rustc_hir::definitions::DefPathData;
use rustc_hir::definitions::DisambiguatedDefPathData;
use rustc_hir::def_id::ModDefId;
use rustc_hir::definitions::DefPathDataName;

use serde::{Serialize, ser::SerializeSeq};
use rustc_utils::source_map::range::CharRange;
use super::*;

pub struct PathDefWithArgs<'tcx> {
    def_id: DefId,
    args: &'tcx [GenericArg<'tcx>],
}

impl<'tcx> PathDefWithArgs<'tcx> {
    pub fn new(def_id: DefId, args: &'tcx [GenericArg<'tcx>]) -> Self {
        PathDefWithArgs { def_id, args, }
    }
}

impl<'tcx> Serialize for PathDefWithArgs<'tcx> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        PathBuilder::def_path(self.def_id, self.args, s)
    }
}

// NOTE: this is the type that the PathBuilder
// will build and serialize.
#[derive(Serialize)]
struct DefinedPath {}


#[derive(Serialize)]
#[serde(tag = "type")]
enum PathSegment<'tcx> {
    Crate,
    Colons,
    LAngle,
    RAngle,
    As,
    Symbol(String),
    ImplAt(CharRange),
    Ty(#[serde(with = "TyDef")] Ty<'tcx>),
    TraitOnlyPath(#[serde(with = "TraitRefPrintOnlyTraitPathDef")] TraitRef<'tcx>),
}

struct PathBuilder<'a, 'tcx: 'a, S: serde::Serializer> {
    infcx: &'a InferCtxt<'tcx>,
    empty_path: bool,
    in_value: bool,
    segments: Vec<PathSegment<'tcx>>,
    _marker: std::marker::PhantomData<S>,
}


impl<'a, 'tcx: 'a, S: serde::Serializer> PathBuilder<'a, 'tcx, S> {
    pub fn def_path(def_id: DefId, args: &'tcx [GenericArg<'tcx>], s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let infcx = super::get_dynamic_ctx();
        let mut builder = PathBuilder {
            infcx,
            empty_path: true,
            in_value: false,
            segments: Vec::new(),
            _marker: std::marker::PhantomData::<S>,
        };
        builder.default_def_path(def_id, args);
        builder.serialize(s)
    }

    fn serialize(self, s: S) -> Result<S::Ok, S::Error> {
        "path::todo!()".serialize(s)
    }

    fn default_def_path(&mut self, def_id: DefId, args: &'tcx [GenericArg<'tcx>]) {
        // TODO: todo!()
    }
}
