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
    if_let_guard
)]

extern crate rustc_apfloat;
extern crate rustc_ast;
extern crate rustc_data_structures;
#[cfg(feature = "testing")]
extern crate rustc_driver;
#[cfg(feature = "testing")]
extern crate rustc_errors;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_hir_analysis;
extern crate rustc_hir_typeck;
extern crate rustc_infer;
#[cfg(feature = "testing")]
extern crate rustc_interface;
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
#[cfg(feature = "testing")]
pub mod test_utils;
#[cfg(test)]
mod ts;
pub mod types;

#[macro_export]
macro_rules! define_idx {
  ($t:ident, $($ty:tt),*) => {
      $(
        index_vec::define_index_type! {
            pub struct $ty = $t;
        }
      )*
      crate::define_tsrs_alias!($($ty,)* => $t);
  }
}

#[macro_export]
macro_rules! define_tsrs_alias {
    ($($($ty:ty,)* => $l:ident),*) => {$($(
        impl ts_rs::TS for $ty {
            const EXPORT_TO: Option<&'static str> =
              Some(concat!("bindings/", stringify!($ty), ".ts"));
            fn name() -> String {
                stringify!($ty).to_owned()
            }
            fn name_with_type_args(args: Vec<String>) -> String {
              assert!(
                  args.is_empty(),
                  "called name_with_type_args on {}",
                  stringify!($ty)
              );
              <$l as ts_rs::TS>::name()
            }
            fn decl() -> String {
              format!(
                "type {}{} = {};",
                stringify!($ty),
                "",
                <$l as ts_rs::TS>::name()
              )
            }
            fn inline() -> String {
              <$l as ts_rs::TS>::name()
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
