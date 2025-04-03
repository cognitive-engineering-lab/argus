//! Remote `serde::Serialize` derives for Rustc types
//!
//! WARNING, these definitions were done hastily, and definitely
//! need a little "fixing up." It will be done at some point.
//! In the meantime, consume at your own risk.
//!
//! Here is a quick guide to naming conventions used in this module. To
//! Serialize types we try to rely on serde::remote when possible.
//! These remote types by convention append a "Def" suffix to the type.
//!
//! For example, `rustc_middle::ty::Ty` is serialized as `TyDef`.
//!
//! Serializing rich source information is *hard*, and requires a step
//! of expansion and processing before all information is had. If you don't
//! believe this claim, take a peek at `rustc_middle::ty::print::pretty` and
//! come back when you're convinced.
//!
//! If a type requires expansion into a richer form, this is done inside the `new` function.
//!
//! If a type needs to be used within a serde `with` attribute, then an associated function
//! `serialize` is defined, and actual serialization will be deferred to the `serialize`
//! extension method.
//!
//! If you need to serialize an optional type then prefix it with `Option__`, and
//! lists of elements are serialized with a prefixed `Slice__`.
#![feature(rustc_private, decl_macro, let_chains)]
#![allow(non_camel_case_types, non_snake_case)]
extern crate rustc_abi;
extern crate rustc_apfloat;
extern crate rustc_ast_ir;
extern crate rustc_data_structures;
extern crate rustc_hir;
extern crate rustc_infer;
extern crate rustc_macros;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_target;
extern crate rustc_trait_selection;

// Make these public to expose the `XXX_Def` wrappers.
pub mod argus;
pub mod r#const;
mod r#dyn;
mod path;
mod safe;
pub mod term;
pub mod ty;
use std::cell::Cell;
pub mod interner;

pub use argus::*;
pub(crate) use argus_ser_macros::{
  argus, serialize_custom_seq, Many, Maybe, Poly,
};
pub(crate) use r#dyn::DynCtxt;
use rustc_infer::infer::InferCtxt;
use rustc_trait_selection::traits::solve::Goal;
// These types are safe for dependents to use.
pub use safe::*;
use serde::Serialize;
use serde_json as json;

use crate::interner::TyInterner;

/// # Panics
///
/// This function expects that serialization succeeded. This usually
/// happens because the *wrong* `InferCtxt` has been passed. Double-check
/// that, then report an issue if it's not the case.
pub fn to_value_expect<'a, 'tcx: 'a, T: Serialize + 'a>(
  infcx: &'a InferCtxt<'tcx>,
  ty_interner: &'a TyInterner<'tcx>,
  value: &T,
) -> json::Value {
  to_value(infcx, ty_interner, value).expect("failed to serialize value")
}

/// Entry function to serialize anything from rustc.
pub fn to_value<'a, 'tcx: 'a, T: Serialize + 'a>(
  infcx: &'a InferCtxt<'tcx>,
  ty_interner: &'a TyInterner<'tcx>,
  value: &T,
) -> Result<json::Value, json::Error> {
  log::trace!("Setting Interner");
  TyInterner::invoke_in(ty_interner, || {
    log::trace!("Setting InferCtxt");
    InferCtxt::invoke_in(infcx, || json::to_value(value))
  })
}

trait InferCtxtSerializeExt {
  fn should_print_verbose(&self) -> bool;
}

impl InferCtxtSerializeExt for InferCtxt<'_> {
  fn should_print_verbose(&self) -> bool {
    self.tcx.sess.verbose_internals()
  }
}

// ----------------------------------------
// Parameters

thread_local! {
    static FORCE_IMPL_FILENAME_LINE: Cell<bool> = const { Cell::new(false) };
    static SHOULD_PREFIX_WITH_CRATE: Cell<bool> = const { Cell::new(false) };
    static NO_TRIMMED_PATH: Cell<bool> = const { Cell::new(false) };
    static FORCE_TRIMMED_PATH: Cell<bool> = const { Cell::new(false) };
    static NO_QUERIES: Cell<bool> = const { Cell::new(false) };
    static NO_VISIBLE_PATH: Cell<bool> = const { Cell::new(false) };
}

macro_rules! define_helper {
    ($($(#[$a:meta])* fn $name:ident($helper:ident, $tl:ident);)+) => {
        $(
            #[must_use]
            pub struct $helper(bool);

            impl $helper {
              #[allow(dead_code)]
                pub fn new() -> $helper {
                    $helper($tl.with(|c| c.replace(true)))
                }
            }

            impl Default for $helper {
              fn default() -> Self {
                Self::new()
              }
            }

            $(#[$a])*
            pub macro $name($e:expr) {
                {
                    let _guard = $helper::new();
                    $e
                }
            }

            impl Drop for $helper {
                fn drop(&mut self) {
                    $tl.with(|c| c.set(self.0))
                }
            }

            pub fn $name() -> bool {
                $tl.with(|c| c.get())
            }
        )+
    }
}

define_helper!(
    /// Avoids running any queries during any prints that occur
    /// during the closure. This may alter the appearance of some
    /// types (e.g. forcing verbose printing for opaque types).
    /// This method is used during some queries (e.g. `explicit_item_bounds`
    /// for opaque types), to ensure that any debug printing that
    /// occurs during the query computation does not end up recursively
    /// calling the same query.
    fn with_no_queries(NoQueriesGuard, NO_QUERIES);
    /// Force us to name impls with just the filename/line number. We
    /// normally try to use types. But at some points, notably while printing
    /// cycle errors, this can result in extra or suboptimal error output,
    /// so this variable disables that check.
    fn with_forced_impl_filename_line(ForcedImplGuard, FORCE_IMPL_FILENAME_LINE);
    /// Adds the `crate::` prefix to paths where appropriate.
    fn with_crate_prefix(CratePrefixGuard, SHOULD_PREFIX_WITH_CRATE);
);

// ----------------------------------------
// Argus index helpers

#[macro_export]
macro_rules! ts {
  ($($ty:ty,)*) => {
    $({
      let error_msg = format!("Failed to export TS binding for type '{}'", stringify!($ty));
      <$ty as ts_rs::TS>::export().expect(error_msg.as_ref());
    })*
  };
}

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

// ---------------------
// Export TS-RS bindings

#[cfg(feature = "testing")]
mod tests {
  #[test]
  fn export_bindings_indices() {
    crate::ts! {
      crate::interner::TyIdx,
    }
  }
}
