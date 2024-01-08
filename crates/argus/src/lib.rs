#![feature(
    rustc_private,
    trait_alias,
    never_type, // proof tree visitor
    min_specialization, // for rustc_index
    let_chains,
    decl_macro // path serialize
)]

extern crate rustc_data_structures;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_hir_analysis;
extern crate rustc_hir_typeck;
extern crate rustc_infer;
extern crate rustc_macros;
extern crate rustc_middle;
extern crate rustc_query_system;
extern crate rustc_serialize;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;
extern crate rustc_trait_selection;
extern crate rustc_type_ir;

pub mod analysis;
pub mod proof_tree;
pub mod serialize;
mod rustc;

// -----------------
// Interfacing types

use rustc_span::symbol::Symbol;
use rustc_middle::ty::Predicate;
use rustc_utils::source_map::range::CharRange;
use serde::Serialize;
use proof_tree::Obligation;
use serialize::ty::{SymbolDef, PredicateDef};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;

#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct AmbiguityError {}

#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[serde(rename_all = "camelCase")]
pub struct TraitError<'tcx> {
  range: CharRange,
  #[serde(with = "PredicateDef")]
  #[cfg_attr(feature = "ts=rs", ts(type = "any"))]
  predicate: Predicate<'tcx>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
pub struct ObligationsInBody<'tcx> {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(serialize_with = "serialize_option")]
  #[cfg_attr(feature = "ts-rs", ts(type = "SymbolDef?"))]
  name: Option<Symbol>,
  range: CharRange,
  ambiguity_errors: Vec<AmbiguityError>,
  trait_errors: Vec<TraitError<'tcx>>,
  #[cfg_attr(feature = "ts-rs", ts(type = "Obligation[]"))]
  obligations: Vec<Obligation<'tcx>>,
}

// Serialize an Option<Symbol> using SymbolDef but the value must be a Some(..)
fn serialize_option<S: serde::Serializer>(
  value: &Option<Symbol>,
  s: S,
) -> Result<S::Ok, S::Error> {
  let Some(symb) = value else {
    unreachable!();
  };

  SymbolDef::serialize(symb, s)
}

// ------------------------------

use anyhow::Result;
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;
use rustc_utils::source_map::range::ToSpan;

#[derive(Debug)]
pub struct Target {
  pub hash: u64,
  pub span: Span,
}

pub trait ToTarget {
  fn to_target(self, tcx: TyCtxt) -> Result<Target>;
}

impl<U: Into<u64>, T: ToSpan> ToTarget for (U, T) {
  fn to_target(self, tcx: TyCtxt) -> Result<Target> {
    self.1.to_span(tcx).map(|span| Target {
      hash: self.0.into(),
      span,
    })
  }
}

// TS-RS exports, these should be moved to a different module. They aren't used now anyways.

// FIXME: this shouldn't currently be used, because we now rely on
// the serialization of rustc types, I need to update the TS-RS
// generation.
#[cfg(test)]
mod tests {

  macro_rules! ts {
      ($($ty:ty,)*) => {
        $({
          let error_msg = format!("Failed to export TS binding for type '{}'", stringify!($ty));
          <$ty as TS>::export().expect(error_msg.as_ref());
        })*
      };
    }

  #[test]
  fn export_bindings_all_tys() {
    ts! {
      // proof_tree::SerializedTree,
      // proof_tree::Node,
      // proof_tree::Obligation,
      // proof_tree::TreeTopology<proof_tree::ProofNodeIdx>,

      // From rustc_utils
      // range::CharRange,
      // range::CharPos,
      // filename::FilenameIndex,
    }
  }
}
