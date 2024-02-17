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

pub struct PathDefNoArgs {
  def_id: DefId,
}

impl PathDefNoArgs {
  pub fn new(def_id: DefId) -> Self {
    Self { def_id }
  }
}

impl Serialize for PathDefNoArgs {
  fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    path_def_no_args(&self.def_id, s)
  }
}

pub(super) fn path_def_no_args<S>(
  def_id: &DefId,
  s: S,
) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  PathBuilder::compile_def_path(*def_id, &[], s)
}

pub struct PathDefWithArgs<'tcx> {
  def_id: DefId,
  args: &'tcx [ty::GenericArg<'tcx>],
}

impl<'tcx> PathDefWithArgs<'tcx> {
  pub fn new(def_id: DefId, args: &'tcx [ty::GenericArg<'tcx>]) -> Self {
    PathDefWithArgs { def_id, args }
  }
}

impl<'tcx> Serialize for PathDefWithArgs<'tcx> {
  fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    PathBuilder::compile_def_path(self.def_id, self.args, s)
  }
}

pub struct AliasPath<'a, 'tcx: 'a> {
  alias_ty: &'a ty::AliasTy<'tcx>,
}

impl<'a, 'tcx: 'a> AliasPath<'a, 'tcx> {
  pub fn new(alias_ty: &'a ty::AliasTy<'tcx>) -> Self {
    AliasPath { alias_ty }
  }
}

impl<'tcx> Serialize for AliasPath<'_, 'tcx> {
  fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    PathBuilder::compile_inherent_projection(self.alias_ty, s)
  }
}

// --------------------------------------------------------
// Value path definitions

pub struct ValuePathWithArgs<'tcx> {
  def_id: DefId,
  args: &'tcx [ty::GenericArg<'tcx>],
}

impl<'tcx> ValuePathWithArgs<'tcx> {
  pub fn new(def_id: DefId, args: &'tcx [ty::GenericArg<'tcx>]) -> Self {
    ValuePathWithArgs { def_id, args }
  }
}

impl<'tcx> Serialize for ValuePathWithArgs<'tcx> {
  fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    PathBuilder::compile_value_path(self.def_id, self.args, s)
  }
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
struct DefinedPath<'tcx>(Vec<PathSegment<'tcx>>);

#[derive(Debug, Serialize)]
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
    #[cfg_attr(feature = "testing", ts(type = "any"))]
    #[serde(with = "serial_ty::TyDef")]
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
    path: Option<TraitRefPrintOnlyTraitPathDefWrapper<'tcx>>,
    #[cfg_attr(feature = "testing", ts(type = "any"))]
    #[serde(with = "TyDef")]
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
      name: name.clone(),
      disambiguator: None,
    }
  }

  pub fn ambiguous_name(name: Symbol, disambiguator: u32) -> Self {
    PathSegment::DefPathDataName {
      name: name.clone(),
      disambiguator: Some(disambiguator),
    }
  }
}

struct PathBuilder<'a, 'tcx: 'a, S: serde::Serializer> {
  infcx: &'a InferCtxt<'tcx>,
  empty_path: bool,
  in_value: bool,
  segments: Vec<PathSegment<'tcx>>,
  _marker: std::marker::PhantomData<S>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
#[serde(tag = "type")]
pub enum CommaSeparatedKind {
  GenericArg,
}

impl<'a, 'tcx: 'a, S: serde::Serializer> PathBuilder<'a, 'tcx, S> {
  // Used for values instead of definition paths, rustc handles them the same.
  pub fn compile_value_path(
    def_id: DefId,
    args: &'tcx [ty::GenericArg<'tcx>],
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self::compile_def_path(def_id, args, s)
  }

  pub fn compile_def_path(
    def_id: DefId,
    args: &'tcx [ty::GenericArg<'tcx>],
    s: S,
  ) -> Result<S::Ok, S::Error>
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

    builder.print_def_path(def_id, args);

    builder.serialize(s)
  }

  pub fn compile_inherent_projection(
    alias_ty: &ty::AliasTy<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
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

    builder.pretty_print_inherent_projection(alias_ty);

    builder.serialize(s)
  }

  fn tcx(&self) -> TyCtxt<'tcx> {
    self.infcx.tcx
  }

  fn serialize(self, s: S) -> Result<S::Ok, S::Error> {
    DefinedPath(self.segments).serialize(s)
  }

  fn should_print_verbose(&self) -> bool {
    self.infcx.should_print_verbose()
  }

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
