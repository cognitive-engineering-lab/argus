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
