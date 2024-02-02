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
    control_flow_enum
)]

extern crate rustc_ast;
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
mod ext;
mod proof_tree;
mod rustc;
mod serialize;
pub mod types;

#[macro_export]
macro_rules! define_idx {
  ($t:ident, $($ty:tt),*) => {
      crate::define_tsrs_alias!($($ty,)* => "number");
      $(
        index_vec::define_index_type! {
            pub struct $ty = $t;
        }
      )*
  }
}

#[macro_export]
macro_rules! define_tsrs_alias {
    ($($($ty:ty,)* => $l:literal),*) => {$($(
        impl ts_rs::TS for $ty {
            fn name() -> String {
                $l.to_owned()
            }
            fn name_with_type_args(args: Vec<String>) -> String {
                assert!(
                    args.is_empty(),
                    "called name_with_type_args on {}",
                    stringify!($ty)
                );
                $l.to_owned()
            }
            fn inline() -> String {
                $l.to_owned()
            }
            fn dependencies() -> Vec<ts_rs::Dependency> {
                vec![]
            }
            fn transparent() -> bool {
                false
            }
        }
    )*)*};
}

#[macro_export]
macro_rules! serialize_as_number {
    (PATH ( $field_path:tt ){ $($name:ident,)* }) => {
        $(
            impl serde::Serialize for $name {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: Serializer,
                {
                    let s = format!("{}", self.$field_path.as_usize());
                    serializer.serialize_str(&s)
                }
            }
        )*
    }
}
