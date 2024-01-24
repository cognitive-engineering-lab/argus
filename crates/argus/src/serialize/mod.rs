//! Remote serde::Serialize derives for Rustc types
//!
//! WARNING, these definitions were done hastily, and definitely
//! need a little "fixing up." It will be done at some point.
//! In the meantime, consume at your own risk.
#![allow(
  non_camel_case_types,
  non_snake_case,
  suspicious_double_ref_op,
  dead_code
)]

pub mod compound;
pub mod r#const;
pub mod path;
pub mod term;
pub mod ty;

use std::cell::Cell;

use r#const::*;
use rustc_infer::infer::InferCtxt;
use rustc_trait_selection::traits::solve::Goal;
use serde::Serialize;
use term::*;
use ty::*;

/// Entry function to serialize anything from rustc.
pub fn serialize_to_value<'a, 'tcx: 'a, T: Serialize + 'a>(
  infcx: &InferCtxt<'tcx>,
  value: &T,
) -> Result<serde_json::Value, serde_json::Error> {
  in_dynamic_ctx(infcx, || serde_json::to_value(&value))
}

// NOTE: setting the dynamic TCX should *only* happen
// before calling the serialize function, it must guarantee
// that the 'tcx lifetime is the same as that of the serialized item.
fluid_let::fluid_let! {static INFCX: &'static InferCtxt<'static>}

fn in_dynamic_ctx<'tcx, T>(
  infcx: &InferCtxt<'tcx>,
  f: impl FnOnce() -> T,
) -> T {
  let infcx: &'static InferCtxt<'static> =
    unsafe { std::mem::transmute(infcx) };
  INFCX.set(infcx, f)
}

fn get_dynamic_ctx<'a, 'tcx: 'a>() -> &'a InferCtxt<'tcx> {
  let infcx: &'static InferCtxt<'static> = INFCX.copied().unwrap();
  unsafe {
    std::mem::transmute::<&'static InferCtxt<'static>, &'a InferCtxt<'tcx>>(
      infcx,
    )
  }
}

trait InferCtxtSerializeExt {
  fn should_print_verbose(&self) -> bool;
}

impl<'tcx> InferCtxtSerializeExt for InferCtxt<'tcx> {
  fn should_print_verbose(&self) -> bool {
    self.tcx.sess.verbose_internals()
  }
}

macro_rules! serialize_custom_seq {
  ($wrap:ident, $serializer:expr, $value:expr) => {{
    let mut seq = $serializer.serialize_seq(Some($value.len()))?;
    for e in $value.iter() {
      seq.serialize_element(&$wrap(e))?;
    }
    seq.end()
  }};
}

pub(crate) use serialize_custom_seq;

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
                pub fn new() -> $helper {
                    $helper($tl.with(|c| c.replace(true)))
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
    /// Prevent path trimming if it is turned on. Path trimming affects `Display` impl
    /// of various rustc types, for example `std::vec::Vec` would be trimmed to `Vec`,
    /// if no other `Vec` is found.
    fn with_no_trimmed_paths(NoTrimmedGuard, NO_TRIMMED_PATH);
    fn with_forced_trimmed_paths(ForceTrimmedGuard, FORCE_TRIMMED_PATH);
    /// Prevent selection of visible paths. `Display` impl of DefId will prefer
    /// visible (public) reexports of types as paths.
    fn with_no_visible_paths(NoVisibleGuard, NO_VISIBLE_PATH);
);
