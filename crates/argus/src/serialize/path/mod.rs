use rustc_hir::{
  def_id::DefId,
  definitions::{DefPathDataName, DisambiguatedDefPathData},
};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::symbol::Symbol;
use rustc_utils::source_map::range::CharRange;
use serde::Serialize;
#[cfg(feature = "testing")]
use ts_rs::TS;

use super::{ty as serial_ty, *};

mod default;
mod pretty;

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct PathDefNoArgs<'tcx>(DefinedPath<'tcx>);

impl<'tcx> PathDefNoArgs<'tcx> {
  pub fn new(def_id: DefId) -> Self {
    Self(PathBuilder::compile_def_path(def_id, &[]))
  }

  pub fn serialize<S>(def_id: &DefId, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self::new(*def_id).serialize(s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct PathDefWithArgs<'tcx>(DefinedPath<'tcx>);
impl<'tcx> PathDefWithArgs<'tcx> {
  pub fn new(def_id: DefId, args: &'tcx [ty::GenericArg<'tcx>]) -> Self {
    Self(PathBuilder::compile_def_path(def_id, args))
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct AliasPath<'tcx>(DefinedPath<'tcx>);
impl<'tcx> AliasPath<'tcx> {
  pub fn new(alias_ty: ty::AliasTy<'tcx>) -> Self {
    Self(PathBuilder::compile_inherent_projection(&alias_ty))
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ValuePathWithArgs<'tcx>(DefinedPath<'tcx>);
impl<'tcx> ValuePathWithArgs<'tcx> {
  pub fn new(def_id: DefId, args: &'tcx [ty::GenericArg<'tcx>]) -> Self {
    Self(PathBuilder::compile_value_path(def_id, args))
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
// Useful in scenarios when using a `ValuePathXXX` would cause the
// pretty printer to enter an infinite loop.
pub struct BasicPathNoArgs<'tcx>(DefinedPath<'tcx>);
impl<'tcx> BasicPathNoArgs<'tcx> {
  pub fn new(def_id: DefId) -> Self {
    Self(PathBuilder::compile_value_path(def_id, &[]))
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
struct DefinedPath<'tcx>(Vec<PathSegment<'tcx>>);

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
#[serde(tag = "type")]
enum PathSegment<'tcx> {
  Colons,     // ::
  LocalCrate, // crate
  RawGuess,   // r#
  DefPathDataName {
    #[cfg_attr(feature = "testing", ts(type = "Symbol"))]
    #[serde(with = "serial_ty::SymbolDef")]
    name: Symbol,
    #[serde(skip_serializing_if = "Option::is_none")]
    disambiguator: Option<u32>,
  },
  Ty {
    #[serde(with = "serial_ty::TyDef")]
    #[cfg_attr(feature = "testing", ts(type = "Ty"))]
    ty: Ty<'tcx>,
  },
  GenericDelimiters {
    inner: Vec<PathSegment<'tcx>>,
  }, // < ... >
  CommaSeparated {
    #[cfg_attr(feature = "testing", ts(type = "any[]"))]
    entries: Vec<serde_json::Value>,
    kind: CommaSeparatedKind,
  }, // ..., ..., ...
  Impl {
    #[cfg_attr(feature = "testing", ts(type = "DefinedPath"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<serial_ty::TraitRefPrintOnlyTraitPathDef<'tcx>>,
    #[cfg_attr(feature = "testing", ts(type = "Ty"))]
    #[serde(with = "serial_ty::TyDef")]
    ty: Ty<'tcx>,
    kind: ImplKind,
  },
  AnonImpl {
    range: CharRange,
  },
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
#[serde(tag = "type")]
pub enum ImplKind {
  As,
  For,
}

impl PathSegment<'_> {
  pub fn unambiguous_name(name: Symbol) -> Self {
    PathSegment::DefPathDataName {
      name,
      disambiguator: None,
    }
  }

  pub fn ambiguous_name(name: Symbol, disambiguator: u32) -> Self {
    PathSegment::DefPathDataName {
      name,
      disambiguator: Some(disambiguator),
    }
  }
}

struct PathBuilder<'a, 'tcx: 'a> {
  infcx: &'a InferCtxt<'tcx>,
  empty_path: bool,
  in_value: bool,
  segments: Vec<PathSegment<'tcx>>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum CommaSeparatedKind {
  GenericArg,
}

impl<'tcx> From<PathBuilder<'_, 'tcx>> for DefinedPath<'tcx> {
  fn from(builder: PathBuilder<'_, 'tcx>) -> Self {
    DefinedPath(builder.segments)
  }
}

impl<'a, 'tcx: 'a> PathBuilder<'a, 'tcx> {
  fn new() -> Self {
    let infcx = super::get_dynamic_ctx();
    PathBuilder {
      infcx,
      empty_path: true,
      in_value: false,
      segments: Vec::new(),
    }
  }

  // Used for values instead of definition paths, rustc handles them the same.
  pub fn compile_value_path(
    def_id: DefId,
    args: &'tcx [ty::GenericArg<'tcx>],
  ) -> DefinedPath<'tcx> {
    Self::compile_def_path(def_id, args)
  }

  pub fn compile_def_path(
    def_id: DefId,
    args: &'tcx [ty::GenericArg<'tcx>],
  ) -> DefinedPath<'tcx> {
    let mut builder = Self::new();
    builder.print_def_path(def_id, args);
    builder.into()
  }

  pub fn compile_inherent_projection(
    alias_ty: &ty::AliasTy<'tcx>,
  ) -> DefinedPath<'tcx> {
    let mut builder = Self::new();
    builder.pretty_print_inherent_projection(alias_ty);
    builder.into()
  }

  fn tcx(&self) -> TyCtxt<'tcx> {
    self.infcx.tcx
  }

  fn should_print_verbose(&self) -> bool {
    self.infcx.should_print_verbose()
  }

  #[allow(dead_code)]
  pub fn print_value_path(
    &mut self,
    def_id: DefId,
    args: &'tcx [ty::GenericArg<'tcx>],
  ) {
    self.print_def_path(def_id, args)
  }

  pub fn fmt_maybe_verbose(
    &mut self,
    data: &DisambiguatedDefPathData,
    _verbose: bool,
  ) {
    match data.data.name() {
      DefPathDataName::Named(name) => {
        self
          .segments
          .push(PathSegment::ambiguous_name(name, data.disambiguator));
        /* CHANGE: if verbose && data.disambiguator != 0 {
          write!(writer, "{}#{}", name, data.disambiguator)
        } else {
          writer.write_str(name.as_str())
        } */
      }
      DefPathDataName::Anon { namespace } => {
        // CHANGE: write!(writer, "{{{}#{}}}", namespace, data.disambiguator)
        self
          .segments
          .push(PathSegment::ambiguous_name(namespace, data.disambiguator));
      }
    }
  }
}
