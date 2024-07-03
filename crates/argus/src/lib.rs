#![feature(
    rustc_private,
    trait_alias,
    never_type, // proof tree visitor
    min_specialization, // for rustc_index
    let_chains,
    decl_macro, // path serialize
    extract_if,
    hash_extract_if,
    box_patterns,
    control_flow_enum,
    if_let_guard,
    lazy_cell
)]
#![warn(clippy::pedantic)]
#![allow(
  clippy::missing_errors_doc,
  clippy::wildcard_imports,
  clippy::must_use_candidate,
  clippy::module_name_repetitions
)]
extern crate rustc_data_structures;
#[cfg(feature = "testing")]
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_hir_typeck;
extern crate rustc_infer;

#[cfg(feature = "testing")]
extern crate rustc_errors;

#[cfg(feature = "testing")]
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_trait_selection;

mod aadebug;
pub mod analysis;
pub mod ext;
pub mod find_bodies; // TODO: remove when upstreamed to rustc-plugin
mod proof_tree;
#[cfg(feature = "testing")]
pub mod test_utils;
mod tls;
pub mod types;

#[cfg(feature = "testing")]
mod tests {
  #[test]
  fn export_bindings_indices() {
    use crate::{proof_tree as pty, types as ty};

    argus_ser::ts! {
      ty::ExprIdx,
      ty::ObligationIdx,

      pty::ProofNodeIdx,
      pty::GoalIdx,
      pty::CandidateIdx,
      pty::ResultIdx,
    }
  }

  #[test]
  fn export_bindings_rustc_utils() {
    use rustc_utils::source_map::{filename as fty, range as uty};

    argus_ser::ts! {
      uty::CharPos,
      uty::CharRange,
      fty::FilenameIndex,
    }
  }
}
