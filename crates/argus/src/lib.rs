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
// #![allow(
//   clippy::missing_errors_doc,
//   clippy::wildcard_imports,
//   clippy::must_use_candidate,
//   clippy::module_name_repetitions
// )]
extern crate rustc_apfloat;
extern crate rustc_ast;
extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_hir_analysis;
extern crate rustc_hir_typeck;
extern crate rustc_infer;
#[cfg(feature = "testing")]
extern crate rustc_interface;
extern crate rustc_macros;
extern crate rustc_metadata;
extern crate rustc_middle;
extern crate rustc_next_trait_solver;
extern crate rustc_query_system;
extern crate rustc_serialize;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;
extern crate rustc_trait_selection;
extern crate rustc_type_ir;

mod aadebug;
pub mod analysis;
pub mod ext;
pub mod find_bodies; // TODO: remove when upstreamed to rustc-plugin
mod proof_tree;
mod rustc;
mod serialize;
#[cfg(feature = "testing")]
pub mod test_utils;
#[cfg(feature = "testing")]
mod ts;
pub mod types;

#[macro_export]
macro_rules! impl_raw_cnv {
  // FIXME: how can I match the type literally here?
  // (usize, $($t:tt)*) => {
  //   index_vec::define_index_type! {
  //     $($t)*
  //   }
  // };
  ($_ty:ty, $($t:tt)*) => {
    index_vec::define_index_type! {
      $($t)*
      // IMPL_RAW_CONVERSION = true;
    }
  };
}

#[macro_export]
macro_rules! define_idx {
  ($t:ident, $($ty:tt),*) => {
      $($crate::impl_raw_cnv! {
          $t,
          pub struct $ty = $t;
        })*
      $crate::define_tsrs_alias!($($ty,)* => $t);
  }
}

#[macro_export]
macro_rules! define_tsrs_alias {
    ($($($ty:ty,)* => $l:ident),*) => {$($(
        #[cfg(feature = "testing")]
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
