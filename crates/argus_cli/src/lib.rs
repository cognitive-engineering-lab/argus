#![feature(rustc_private, trait_alias, associated_type_defaults)]

extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_macros;
extern crate rustc_middle;
extern crate rustc_serialize;
extern crate rustc_span;

pub mod plugin;
pub use plugin::ArgusPlugin;
