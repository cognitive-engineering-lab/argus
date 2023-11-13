#![feature(
    rustc_private,
    trait_alias,
    never_type
)]

extern crate rustc_data_structures;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_hir_analysis;
extern crate rustc_hir_typeck;
extern crate rustc_infer;
extern crate rustc_middle;
extern crate rustc_trait_selection;
extern crate rustc_type_ir;

pub mod analysis;
pub mod proof_tree;
