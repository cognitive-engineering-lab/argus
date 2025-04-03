#![feature(rustc_private, let_chains, box_patterns)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions, clippy::missing_errors_doc)]
extern crate rustc_data_structures;
extern crate rustc_hashes;
extern crate rustc_hir;
extern crate rustc_hir_typeck;
extern crate rustc_infer;
extern crate rustc_middle;
extern crate rustc_next_trait_solver;
extern crate rustc_query_system;
extern crate rustc_span;
extern crate rustc_trait_selection;
extern crate rustc_type_ir;

pub mod hash;
pub mod infer;
pub mod iter;
// Most of the rustc code is copied from private rustc modules
// and it's not worth fixing all the clippy warnings.
#[allow(clippy::pedantic)]
pub mod rustc;
pub mod ty;
pub mod utils;

use rustc_trait_selection::traits::{query::NoSolution, solve::Certainty};
type EvaluationResult = Result<Certainty, NoSolution>;

#[cfg(test)]
mod tests {
  use serde::Serialize;
  use serde_json;

  #[test]
  /// NOTE the Argus serialization depends on the fact that a
  /// two field tuple struct (and a skipped field) with `transparent` will produce
  /// the first value bare.
  fn serde_transparent() {
    #[derive(Serialize)]
    #[serde(transparent)]
    pub struct W<'a>(i32, #[serde(skip)] PhantomData<&'a ()>);

    impl<'a> W<'a> {
      fn new(v: i32) -> Self {
        W(v, PhantomData)
      }
    }

    let s = serde_json::to_string(&W::new(0)).unwrap();
    assert!(s.eq("0"));
  }
}
