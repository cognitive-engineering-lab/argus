use std::cell::Cell;

use rustc_hir::{
  def::DefKind,
  def_id::{CrateNum, DefId, ModDefId, LOCAL_CRATE},
  definitions::{
    DefKey, DefPathData, DefPathDataName, DisambiguatedDefPathData,
  },
};
use rustc_middle::ty::{self, print as rustc_print, *};
use rustc_session::cstore::{ExternCrate, ExternCrateSource};
use rustc_span::symbol::{kw, Ident, Symbol};

use rustc_utils::source_map::range::CharRange;
use serde::Serialize;
use log::debug;

use super::*;

mod pretty;
mod default;

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
  args: &'tcx [GenericArg<'tcx>],
}

impl<'tcx> PathDefWithArgs<'tcx> {
  pub fn new(def_id: DefId, args: &'tcx [GenericArg<'tcx>]) -> Self {
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

// --------------------------------------------------------
// Value path definitions

pub struct ValuePathWithArgs<'tcx> {
  def_id: DefId,
  args: &'tcx [GenericArg<'tcx>],
}

impl<'tcx> ValuePathWithArgs<'tcx> {
  pub fn new(def_id: DefId, args: &'tcx [GenericArg<'tcx>]) -> Self {
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

// NOTE: this is the type that the PathBuilder
// will build and serialize.
#[derive(Serialize)]
struct DefinedPath {}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum PathSegment<'tcx> {
  Colons,     // ::
  LocalCrate, // crate
  RawGuess,   // r#
  DefPathDataName {
    #[serde(with = "SymbolDef")]
    name: Symbol,
    #[serde(skip_serializing_if = "Option::is_none")]
    disambiguator: Option<u32>,
  },
  Crate {
    #[serde(with = "SymbolDef")]
    name: Symbol,
  },
  Ty {
    #[serde(with = "TyDef")]
    ty: Ty<'tcx>,
  },
  GenericDelimiters {
    inner: Vec<PathSegment<'tcx>>,
  }, // < ... >
  CommaSeparated {
    entries: Vec<serde_json::Value>,
    kind: CommaSeparatedKind,
  }, // ..., ..., ...
  Impl {
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<TraitRefPrintOnlyTraitPathDefWrapper<'tcx>>,
    #[serde(with = "TyDef")]
    ty: Ty<'tcx>,
    kind: ImplKind,
  },
  AnonImpl {
    range: CharRange,
  },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CommaSeparatedKind {
  GenericArg,
}

impl<'a, 'tcx: 'a, S: serde::Serializer> PathBuilder<'a, 'tcx, S> {
  // Used for values instead of definition paths, rustc handles them the same.
  pub fn compile_value_path(
    def_id: DefId,
    args: &'tcx [GenericArg<'tcx>],
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self::compile_def_path(def_id, args, s)
  }

  pub fn compile_def_path(
    def_id: DefId,
    args: &'tcx [GenericArg<'tcx>],
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

  fn tcx(&self) -> TyCtxt<'tcx> {
    self.infcx.tcx
  }

  fn serialize(self, s: S) -> Result<S::Ok, S::Error> {
    self.segments.serialize(s)
  }

  fn should_print_verbose(&self) -> bool {
    self.infcx.should_print_verbose()
  }


  pub fn print_value_path(&mut self, def_id: DefId, args: &'tcx [GenericArg<'tcx>]) {
    self.print_def_path(def_id, args)
  }


  pub fn fmt_maybe_verbose(&mut self, data: &DisambiguatedDefPathData, verbose: bool) {
    match data.data.name() {
        DefPathDataName::Named(name) => {
            self.segments.push(PathSegment::ambiguous_name(name, data.disambiguator));
            /* CHANGE: if verbose && data.disambiguator != 0 {
              write!(writer, "{}#{}", name, data.disambiguator)
            } else {
              writer.write_str(name.as_str())
            } */
        }
        DefPathDataName::Anon { namespace } => {
            todo!();
            // write!(writer, "{{{}#{}}}", namespace, data.disambiguator)
        }
    }
}

}

// pub trait Printer<'tcx>: Sized {
//   fn tcx<'a>(&'a self) -> TyCtxt<'tcx>;

//   fn print_def_path(
//       &mut self,
//       def_id: DefId,
//       args: &'tcx [GenericArg<'tcx>],
//   ) -> Result<(), PrintError> {
//       self.default_print_def_path(def_id, args)
//   }

//   fn print_impl_path(
//       &mut self,
//       impl_def_id: DefId,
//       args: &'tcx [GenericArg<'tcx>],
//       self_ty: Ty<'tcx>,
//       trait_ref: Option<ty::TraitRef<'tcx>>,
//   ) -> Result<(), PrintError> {
//       self.default_print_impl_path(impl_def_id, args, self_ty, trait_ref)
//   }

//   fn print_region(&mut self, region: ty::Region<'tcx>) -> Result<(), PrintError>;

//   fn print_type(&mut self, ty: Ty<'tcx>) -> Result<(), PrintError>;

//   fn print_dyn_existential(
//       &mut self,
//       predicates: &'tcx ty::List<ty::PolyExistentialPredicate<'tcx>>,
//   ) -> Result<(), PrintError>;

//   fn print_const(&mut self, ct: ty::Const<'tcx>) -> Result<(), PrintError>;

//   fn path_crate(&mut self, cnum: CrateNum) -> Result<(), PrintError>;

//   fn path_qualified(
//       &mut self,
//       self_ty: Ty<'tcx>,
//       trait_ref: Option<ty::TraitRef<'tcx>>,
//   ) -> Result<(), PrintError>;

//   fn path_append_impl(
//       &mut self,
//       print_prefix: impl FnOnce(&mut Self) -> Result<(), PrintError>,
//       disambiguated_data: &DisambiguatedDefPathData,
//       self_ty: Ty<'tcx>,
//       trait_ref: Option<ty::TraitRef<'tcx>>,
//   ) -> Result<(), PrintError>;

//   fn path_append(
//       &mut self,
//       print_prefix: impl FnOnce(&mut Self) -> Result<(), PrintError>,
//       disambiguated_data: &DisambiguatedDefPathData,
//   ) -> Result<(), PrintError>;

//   fn path_generic_args(
//       &mut self,
//       print_prefix: impl FnOnce(&mut Self) -> Result<(), PrintError>,
//       args: &[GenericArg<'tcx>],
//   ) -> Result<(), PrintError>;

//   // Defaults (should not be overridden):

//   #[instrument(skip(self), level = "debug")]
//   fn default_print_def_path(
//       &mut self,
//       def_id: DefId,
//       args: &'tcx [GenericArg<'tcx>],
//   ) -> Result<(), PrintError>;

//   fn default_print_impl_path(
//       &mut self,
//       impl_def_id: DefId,
//       _args: &'tcx [GenericArg<'tcx>],
//       self_ty: Ty<'tcx>,
//       impl_trait_ref: Option<ty::TraitRef<'tcx>>,
//   ) -> Result<(), PrintError>; 
// }