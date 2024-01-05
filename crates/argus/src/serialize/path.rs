use std::cell::Cell;

use rustc_type_ir as ir;
use rustc_middle::{ty::{self, *, abstract_const::CastKind}, mir::{BinOp, UnOp}};
use rustc_hir::def_id::{DefId, DefIndex, CrateNum};
use rustc_data_structures::sso::SsoHashSet;
use rustc_span::{symbol::{kw, sym, Ident, Symbol}, Span};
use rustc_target::spec::abi::Abi;
use rustc_session::cstore::ExternCrateSource;
use rustc_middle::ty::print as rustc_print;
use rustc_session::cstore::ExternCrate;
use rustc_hir::Unsafety;
use rustc_hir::definitions::DefPathData;
use rustc_hir::definitions::DisambiguatedDefPathData;
use rustc_hir::definitions::DefPathDataName;
use rustc_hir::definitions::DefKey;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_hir::def_id::ModDefId;

use serde::{Serialize, ser::SerializeSeq};
use rustc_utils::source_map::range::CharRange;
use log::debug;
use super::*;

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

// --------------------------------------------------------

pub(super) fn path_def_no_args<S>(def_id: DefId, s: S) -> Result<S::Ok, S::Error> 
where
    S: serde::Serializer,
{
    PathBuilder::compile_def_path(def_id, &[], s)
}

pub struct PathDefWithArgs<'tcx> {
    def_id: DefId,
    args: &'tcx [GenericArg<'tcx>],
}

impl<'tcx> PathDefWithArgs<'tcx> {
    pub fn new(def_id: DefId, args: &'tcx [GenericArg<'tcx>]) -> Self {
        PathDefWithArgs { def_id, args, }
    }
}

impl<'tcx> Serialize for PathDefWithArgs<'tcx> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        PathBuilder::compile_def_path(self.def_id, self.args, s)
    }
}

// NOTE: this is the type that the PathBuilder
// will build and serialize.
#[derive(Serialize)]
struct DefinedPath {}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum PathSegment<'tcx> {
    Colons,     // ::
    LocalCrate, // crate
    RawGuess,   // r#
    Symbol {
        #[serde(with = "SymbolDef")]
        name: Symbol,
    },
    DefPathDataName {
        #[serde(with = "SymbolDef")]
        name: Symbol,
        disambiguator: u32,
    },
    Crate {
        #[serde(with = "SymbolDef")]
        name: Symbol
    },
    Ty {
        #[serde(with = "TyDef")]
        ty: Ty<'tcx>,
    },
    GenericDelimiters {
        inner: Vec<PathSegment<'tcx>>
    },  // < ... >
    CommaSeparated {
        entries: Vec<serde_json::Value>,
        kind: CommaSeparatedKind,
    }, // ..., ..., ...
    Impl {
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<TraitRefPrintOnlyTraitPathDefWrapper<'tcx>>,
        #[serde(with = "TyDef")]
        ty: Ty<'tcx>,
        kind: ImplKind,
    },
    AnonImpl {
        range: CharRange,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ImplKind {
    As,
    For,
}

struct PathBuilder<'a, 'tcx: 'a, S: serde::Serializer> {
    infcx: &'a InferCtxt<'tcx>,
    empty_path: bool,
    in_value: bool,
    segments: Vec<PathSegment<'tcx>>,
    _marker: std::marker::PhantomData<S>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CommaSeparatedKind {
    GenericArg,
}

impl<'a, 'tcx: 'a, S: serde::Serializer> PathBuilder<'a, 'tcx, S> {
    // Used for values instead of definition paths, rustc handles them the same.
    pub fn compile_value_path(def_id: DefId, args: &'tcx [GenericArg<'tcx>], s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Self::compile_def_path(def_id, args, s)
    }

    pub fn compile_def_path(def_id: DefId, args: &'tcx [GenericArg<'tcx>], s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let infcx = super::get_dynamic_ctx();
        let mut builder = PathBuilder {
            infcx,
            empty_path: true,
            in_value: false,
            segments: Vec::new(),
            _marker: std::marker::PhantomData::<S>,
        };

        // HACK: I don't think we actually want to trim
        // anything, so I'll just disable it here,
        // XXX can be removed in the future.
        with_no_trimmed_paths!{
            with_no_visible_paths!{
                builder.def_path(def_id, args)
            }
        };

        builder.serialize(s)
    }

    fn tcx(&self) -> TyCtxt<'tcx> {
        self.infcx.tcx
    }

    fn serialize(self, s: S) -> Result<S::Ok, S::Error> {
        debug!("Serializing segments {:#?}", self.segments);
        self.segments.serialize(s)
    }

    pub fn value_path(&mut self, def_id: DefId, args: &'tcx [GenericArg<'tcx>]) {
        self.def_path(def_id, args)
    }

    // Equivalent to rustc_middle::ty::print::pretty::def_path
    // This entry point handles the other grueling cases and handling overflow,
    // which apparently is a thing in pretty printing! (see issues #55779 and #87932).
    // Equivalent to [rustc_middle::ty::print::pretty::print_def_path].
    fn def_path(&mut self, def_id: DefId, args: &'tcx [GenericArg<'tcx>]) {
        if args.is_empty() {
            if self.try_trimmed_def_path(def_id) {
                return;
            }

            if self.try_visible_def_path(def_id) {
                return;
            }
        }

        let key = self.tcx().def_key(def_id);
        if let DefPathData::Impl = key.disambiguated_data.data {
            // Always use types for non-local impls, where types are always
            // available, and filename/line-number is mostly uninteresting.
            let use_types = !def_id.is_local() || {
                // Otherwise, use filename/line-number if forced.
                let force_no_types = with_forced_impl_filename_line();
                !force_no_types
            };

            if !use_types {
                // If no type info is available, fall back to
                // pretty printing some span information. This should
                // only occur very early in the compiler pipeline.
                let parent_def_id = DefId { index: key.parent.unwrap(), ..def_id };
                let span = self.tcx().def_span(def_id);

                self.def_path(parent_def_id, &[]);

                // HACK(eddyb) copy of `path_append` to avoid
                // constructing a `DisambiguatedDefPathData`.
                if !self.empty_path {
                    self.segments.push(PathSegment::Colons);
                }
                let Ok(impl_range) = CharRange::from_span(span, self.tcx().sess.source_map()) else {
                    todo!("impl span not converted to `CharRange`");
                };

                self.segments.push(PathSegment::AnonImpl {
                    range: impl_range,
                });
                self.empty_path = false;

                return;
            }
        }

        self.default_def_path(def_id, args)
    }

    fn force_trimmed_def_path(&mut self, def_id: DefId) -> bool {
        let key = self.tcx().def_key(def_id);
        let visible_parent_map = self.tcx().visible_parent_map(());
        let kind = self.tcx().def_kind(def_id);

        let get_local_name = |this: &Self, name, def_id, key: DefKey| {
            if let Some(visible_parent) = visible_parent_map.get(&def_id)
                && let actual_parent = this.tcx().opt_parent(def_id)
                && let DefPathData::TypeNs(_) = key.disambiguated_data.data
                && Some(*visible_parent) != actual_parent
            {
                this.tcx()
                // FIXME(typed_def_id): Further propagate ModDefId
                    .module_children(ModDefId::new_unchecked(*visible_parent))
                    .iter()
                    .filter(|child| child.res.opt_def_id() == Some(def_id))
                    .find(|child| child.vis.is_public() && child.ident.name != kw::Underscore)
                    .map(|child| child.ident.name)
                    .unwrap_or(name)
            } else {
                name
            }
        };

        if let DefKind::Variant = kind
            && let Some(symbol) = self.tcx().trimmed_def_paths(()).get(&def_id)
        {
            // If `Assoc` is unique, we don't want to talk about `Trait::Assoc`.
            self.segments.push(PathSegment::Symbol {
                name: get_local_name(self, *symbol, def_id, key),
            });
            return true;

        }

        if let Some(symbol) = key.get_opt_name() {
            if let DefKind::AssocConst | DefKind::AssocFn | DefKind::AssocTy = kind
                && let Some(parent) = self.tcx().opt_parent(def_id)
                && let parent_key = self.tcx().def_key(parent)
                && let Some(symbol) = parent_key.get_opt_name()
            {
                // Trait
                self.segments.push(PathSegment::Symbol {
                    name: get_local_name(self, symbol, parent, parent_key)
                });
                self.segments.push(PathSegment::Colons);

            } else if let DefKind::Variant = kind
                && let Some(parent) = self.tcx().opt_parent(def_id)
                && let parent_key = self.tcx().def_key(parent)
                && let Some(symbol) = parent_key.get_opt_name()
            {
                // Enum

                // For associated items and variants, we want the "full" path, namely, include
                // the parent type in the path. For example, `Iterator::Item`.
                self.segments.push(PathSegment::Symbol {
                    name: get_local_name(self, symbol, parent, parent_key)
                });
                self.segments.push(PathSegment::Colons);

            } else if let DefKind::Struct
                | DefKind::Union
                | DefKind::Enum
                | DefKind::Trait
                | DefKind::TyAlias
                | DefKind::Fn
                | DefKind::Const
                | DefKind::Static(_) = kind
            { /* intentionally blank */ } else {
                // If not covered above, like for example items out of `impl` blocks, fallback.
                return false;
            }
            self.segments.push(
                PathSegment::Symbol {
                    name: get_local_name(self, symbol, def_id, key)
                }
            );
            return true;
        }
        false
    }

    fn try_trimmed_def_path(&mut self, def_id: DefId) -> bool {
        if with_forced_trimmed_paths() {
            let trimmed = self.force_trimmed_def_path(def_id);
            if trimmed {
                return true;
            }
        }
        if
            // !self.tcx().sess.opts.unstable_opts.trim_diagnostic_paths
            // || matches!(self.tcx().sess.opts.trimmed_def_paths, TrimmedDefPaths::Never)
            // ||
            with_no_trimmed_paths()
            || with_crate_prefix()
        {
            return false;
        }

        match self.tcx().trimmed_def_paths(()).get(&def_id) {
            None => false,
            Some(symbol) => {
                self.segments.push(PathSegment::Symbol{ name: *symbol });
                // write!(self, "{}", Ident::with_dummy_span(*symbol))?;
                true
            }
        }
    }

    fn try_visible_def_path_recur(&mut self, def_id: DefId, callers: &mut Vec<DefId>) -> bool {
        debug!("try_visible_def_path: def_id={:?}", def_id);

        // If `def_id` is a direct or injected extern crate, return the
        // path to the crate followed by the path to the item within the crate.
        if let Some(cnum) = def_id.as_crate_root() {
            if cnum == LOCAL_CRATE {
                self.path_crate(cnum);
                return true;
            }

            // In local mode, when we encounter a crate other than
            // LOCAL_CRATE, execution proceeds in one of two ways:
            //
            // 1. For a direct dependency, where user added an
            //    `extern crate` manually, we put the `extern
            //    crate` as the parent. So you wind up with
            //    something relative to the current crate.
            // 2. For an extern inferred from a path or an indirect crate,
            //    where there is no explicit `extern crate`, we just prepend
            //    the crate name.
            match self.tcx().extern_crate(def_id) {
                Some(&ExternCrate { src, dependency_of, span, .. }) => match (src, dependency_of) {
                    (ExternCrateSource::Extern(def_id), LOCAL_CRATE) => {
                        // NOTE(eddyb) the only reason `span` might be dummy,
                        // that we're aware of, is that it's the `std`/`core`
                        // `extern crate` injected by default.
                        // FIXME(eddyb) find something better to key this on,
                        // or avoid ending up with `ExternCrateSource::Extern`,
                        // for the injected `std`/`core`.
                        if span.is_dummy() {
                            self.path_crate(cnum);
                            return true;
                        }

                        // Disable `try_print_trimmed_def_path` behavior within
                        // the `print_def_path` call, to avoid infinite recursion
                        // in cases where the `extern crate foo` has non-trivial
                        // parents, e.g. it's nested in `impl foo::Trait for Bar`
                        // (see also issues #55779 and #87932).
                        with_no_visible_paths!(self.def_path(def_id, &[]));

                        return true;
                    }
                    (ExternCrateSource::Path, LOCAL_CRATE) => {
                        self.path_crate(cnum);
                        return true;
                    }
                    _ => {}
                },
                None => {
                    self.path_crate(cnum);
                    return true;
                }
            }
        }

        if def_id.is_local() {
            return false;
        }

        let visible_parent_map = self.tcx().visible_parent_map(());

        let mut cur_def_key = self.tcx().def_key(def_id);
        debug!("try_visible_def_path: cur_def_key={:?}", cur_def_key);

        // For a constructor, we want the name of its parent rather than <unnamed>.
        if let DefPathData::Ctor = cur_def_key.disambiguated_data.data {
            let parent = DefId {
                krate: def_id.krate,
                index: cur_def_key
                    .parent
                    .expect("`DefPathData::Ctor` / `VariantData` missing a parent"),
            };

            cur_def_key = self.tcx().def_key(parent);
        }

        let Some(visible_parent) = visible_parent_map.get(&def_id).cloned() else {
            return false;
        };

        let actual_parent = self.tcx().opt_parent(def_id);
        debug!(
            "try_visible_def_path: visible_parent={:?} actual_parent={:?}",
            visible_parent, actual_parent,
        );

        let mut data = cur_def_key.disambiguated_data.data;
        debug!(
            "try_visible_def_path: data={:?} visible_parent={:?} actual_parent={:?}",
            data, visible_parent, actual_parent,
        );

        match data {
            // In order to output a path that could actually be imported (valid and visible),
            // we need to handle re-exports correctly.
            //
            // For example, take `std::os::unix::process::CommandExt`, this trait is actually
            // defined at `std::sys::unix::ext::process::CommandExt` (at time of writing).
            //
            // `std::os::unix` reexports the contents of `std::sys::unix::ext`. `std::sys` is
            // private so the "true" path to `CommandExt` isn't accessible.
            //
            // In this case, the `visible_parent_map` will look something like this:
            //
            // (child) -> (parent)
            // `std::sys::unix::ext::process::CommandExt` -> `std::sys::unix::ext::process`
            // `std::sys::unix::ext::process` -> `std::sys::unix::ext`
            // `std::sys::unix::ext` -> `std::os`
            //
            // This is correct, as the visible parent of `std::sys::unix::ext` is in fact
            // `std::os`.
            //
            // When printing the path to `CommandExt` and looking at the `cur_def_key` that
            // corresponds to `std::sys::unix::ext`, we would normally print `ext` and then go
            // to the parent - resulting in a mangled path like
            // `std::os::ext::process::CommandExt`.
            //
            // Instead, we must detect that there was a re-export and instead print `unix`
            // (which is the name `std::sys::unix::ext` was re-exported as in `std::os`). To
            // do this, we compare the parent of `std::sys::unix::ext` (`std::sys::unix`) with
            // the visible parent (`std::os`). If these do not match, then we iterate over
            // the children of the visible parent (as was done when computing
            // `visible_parent_map`), looking for the specific child we currently have and then
            // have access to the re-exported name.
            DefPathData::TypeNs(ref mut name) if Some(visible_parent) != actual_parent => {
                // Item might be re-exported several times, but filter for the one
                // that's public and whose identifier isn't `_`.
                let reexport = self
                    .tcx()
                    // FIXME(typed_def_id): Further propagate ModDefId
                    .module_children(ModDefId::new_unchecked(visible_parent))
                    .iter()
                    .filter(|child| child.res.opt_def_id() == Some(def_id))
                    .find(|child| child.vis.is_public() && child.ident.name != kw::Underscore)
                    .map(|child| child.ident.name);

                if let Some(new_name) = reexport {
                    *name = new_name;
                } else {
                    // There is no name that is public and isn't `_`, so bail.
                    return false;
                }
            }
            // Re-exported `extern crate` (#43189).
            DefPathData::CrateRoot => {
                data = DefPathData::TypeNs(self.tcx().crate_name(def_id.krate));
            }
            _ => {}
        }
        debug!("try_visible_def_path: data={:?}", data);

        if callers.contains(&visible_parent) {
            return false;
        }
        callers.push(visible_parent);
        // HACK(eddyb) this bypasses `path_append`'s prefix printing to avoid
        // knowing ahead of time whether the entire path will succeed or not.
        // To support printers that do not implement `PrettyPrinter`, a `Vec` or
        // linked list on the stack would need to be built, before any printing.
        match self.try_visible_def_path_recur(visible_parent, callers) {
            false => return false,
            true => {}
        }
        callers.pop();
        self.path_append(|_| (), &DisambiguatedDefPathData { data, disambiguator: 0 });
        true
    }

    fn try_visible_def_path(&mut self, def_id: DefId) -> bool {
        if with_no_visible_paths() {
            return false;
        }

        let mut callers = Vec::new();
        self.try_visible_def_path_recur(def_id, &mut callers)
    }

    // Equivalent to rustc_middle::ty::print::default_def_path
    fn default_def_path(&mut self, def_id: DefId, args: &'tcx [GenericArg<'tcx>]) {
        let key = self.tcx().def_key(def_id);
        debug!("{:?}", key);

        match key.disambiguated_data.data {
            DefPathData::CrateRoot => {
                assert!(key.parent.is_none());
                self.path_crate(def_id.krate)
            }

            DefPathData::Impl => {
                let generics = self.tcx().generics_of(def_id);
                let self_ty = self.tcx().type_of(def_id);
                let impl_trait_ref = self.tcx().impl_trait_ref(def_id);
                let (self_ty, impl_trait_ref) = if args.len() >= generics.count() {
                    (
                        self_ty.instantiate(self.tcx(), args),
                        impl_trait_ref.map(|i| i.instantiate(self.tcx(), args)),
                    )
                } else {
                    (
                        self_ty.instantiate_identity(),
                        impl_trait_ref.map(|i| i.instantiate_identity()),
                    )
                };
                self.impl_path(def_id, args, self_ty, impl_trait_ref)
            }

            _ => {
                let parent_def_id = DefId { index: key.parent.unwrap(), ..def_id };

                let mut parent_args = args;
                let mut trait_qualify_parent = false;
                if !args.is_empty() {
                    let generics = self.tcx().generics_of(def_id);
                    parent_args = &args[..generics.parent_count.min(args.len())];

                    match key.disambiguated_data.data {
                        // Closures' own generics are only captures, don't print them.
                        DefPathData::Closure => {}
                        // This covers both `DefKind::AnonConst` and `DefKind::InlineConst`.
                        // Anon consts doesn't have their own generics, and inline consts' own
                        // generics are their inferred types, so don't print them.
                        DefPathData::AnonConst => {}

                        // If we have any generic arguments to print, we do that
                        // on top of the same path, but without its own generics.
                        _ => {
                            if false && /* <-- TODO(gavinleroy) */!generics.params.is_empty() && args.len() >= generics.count() {
                                let args = generics.own_args_no_defaults(self.tcx(), args);
                                return self.path_generic_args(
                                    |cx| cx.def_path(def_id, parent_args),
                                    args,
                                );
                            }
                        }
                    }

                    // FIXME(eddyb) try to move this into the parent's printing
                    // logic, instead of doing it when printing the child.
                    trait_qualify_parent = generics.has_self
                        && generics.parent == Some(parent_def_id)
                        && parent_args.len() == generics.parent_count
                        && self.tcx().generics_of(parent_def_id).parent_count == 0;
                }

                self.path_append(
                    |cx: &mut Self| {
                        if trait_qualify_parent {
                            let trait_ref = ty::TraitRef::new(
                                cx.tcx(),
                                parent_def_id,
                                parent_args.iter().copied(),
                            );
                            cx.path_qualified(trait_ref.self_ty(), Some(trait_ref))
                        } else {
                            cx.def_path(parent_def_id, parent_args)
                        }
                    },
                    &key.disambiguated_data,
                )
            }
        }
    }

    fn path_crate(&mut self, cnum: CrateNum) {
        debug!("Path crate {:?}", cnum);
        self.empty_path = true;
        if cnum == LOCAL_CRATE {
            if self.tcx().sess.at_least_rust_2018() {
                // We add the `crate::` keyword on Rust 2018, only when desired.
                if with_crate_prefix() {
                    self.segments.push(PathSegment::LocalCrate);
                    self.empty_path = false;
                }
            }

        } else {
            let name = self.tcx().crate_name(cnum);
            self.segments.push(PathSegment::Crate { name });
            self.empty_path = false;
        }
    }

    fn impl_path(
        &mut self,
        impl_def_id: DefId,
        args: &'tcx [GenericArg<'tcx>],
        self_ty: Ty<'tcx>,
        trait_ref: Option<ty::TraitRef<'tcx>>,
    ) {
        self.default_impl_path(impl_def_id, args, self_ty, trait_ref)
    }

    fn default_impl_path(
        &mut self,
        impl_def_id: DefId,
        _args: &'tcx [GenericArg<'tcx>],
        self_ty: Ty<'tcx>,
        impl_trait_ref: Option<ty::TraitRef<'tcx>>,
    ) {
        debug!(
            "impl_path: impl_def_id={:?}, self_ty={}, impl_trait_ref={:?}",
            impl_def_id, self_ty, impl_trait_ref
        );

        let key = self.tcx().def_key(impl_def_id);
        let parent_def_id = DefId { index: key.parent.unwrap(), ..impl_def_id };

        // Decide whether to print the parent path for the impl.
        // Logically, since impls are global, it's never needed, but
        // users may find it useful. Currently, we omit the parent if
        // the impl is either in the same module as the self-type or
        // as the trait.
        let in_self_mod = match rustc_print::characteristic_def_id_of_type(self_ty) {
            None => false,
            Some(ty_def_id) => self.tcx().parent(ty_def_id) == parent_def_id,
        };
        let in_trait_mod = match impl_trait_ref {
            None => false,
            Some(trait_ref) => self.tcx().parent(trait_ref.def_id) == parent_def_id,
        };

        if !in_self_mod && !in_trait_mod {
            // If the impl is not co-located with either self-type or
            // trait-type, then fallback to a format that identifies
            // the module more clearly.
            self.path_append_impl(
                |cx| cx.def_path(parent_def_id, &[]),
                &key.disambiguated_data,
                self_ty,
                impl_trait_ref,
            )
        } else {
            // Otherwise, try to give a good form that would be valid language
            // syntax. Preferably using associated item notation.
            self.path_qualified(self_ty, impl_trait_ref)
        }
    }

    fn path_append(
        &mut self,
        prefix: impl FnOnce(&mut Self),
        disambiguated_data: &DisambiguatedDefPathData,
    ) {
        prefix(self);

        // Skip `::{{extern}}` blocks and `::{{constructor}}` on tuple/unit structs.
        if let DefPathData::ForeignMod | DefPathData::Ctor = disambiguated_data.data {
            return;
        }

        let name = disambiguated_data.data.name();
        if !self.empty_path {
            self.segments.push(PathSegment::Colons);
        }

        if let DefPathDataName::Named(name) = name {
            if Ident::with_dummy_span(name).is_raw_guess() {
                self.segments.push(PathSegment::RawGuess);
            }
        }

        self.disambiguated_def_path_data(disambiguated_data, true);
        self.empty_path = false;
    }

    fn disambiguated_def_path_data(&mut self, disambiguated_data: &DisambiguatedDefPathData, _verbose: bool) {
        let name = match disambiguated_data.data.name() {
            DefPathDataName::Named(name) => name, 
            DefPathDataName::Anon { namespace } => namespace, 
        };
        self.segments.push(PathSegment::DefPathDataName {
            name,
            disambiguator: disambiguated_data.disambiguator,
        });
    }

    fn path_append_impl(
        &mut self,
        prefix: impl FnOnce(&mut Self),
        _disambiguated_data: &DisambiguatedDefPathData,
        self_ty: Ty<'tcx>,
        trait_ref: Option<ty::TraitRef<'tcx>>,
    ) {
        debug!("path_append_impl  {:?} {:?}", self_ty, trait_ref);
        self.pretty_path_append_impl(
            |cx| {
                prefix(cx);
                if !cx.empty_path {
                    cx.segments.push(PathSegment::Colons);
                }
            },
            self_ty,
            trait_ref,
        );
        self.empty_path = false;
    }

    fn pretty_path_append_impl(
        &mut self,
        prefix: impl FnOnce(&mut Self),
        self_ty: Ty<'tcx>,
        trait_ref: Option<ty::TraitRef<'tcx>>,
    ) {
        prefix(self);

        self.generic_delimiters(|cx| {
            cx.segments.push(PathSegment::Impl {
                path: trait_ref.map(|t| TraitRefPrintOnlyTraitPathDefWrapper(t)),
                ty: self_ty,
                kind: ImplKind::For,
            });
        })
    }

    fn generic_delimiters(
        &mut self,
        f: impl FnOnce(&mut Self),
    ) {
        let previous_segments = std::mem::take(&mut self.segments);
        let was_in_value = std::mem::replace(&mut self.in_value, false);
        f(self); // Recursively compile the path.
        self.in_value = was_in_value;
        let inner = std::mem::replace(&mut self.segments, previous_segments);
        self.segments.push(PathSegment::GenericDelimiters { inner });
    }

    fn comma_sep<T>(&mut self, mut elems: impl Iterator<Item = T>, kind: CommaSeparatedKind) 
    where
        T: Serialize,
    {
        let entries = elems
            .map(|elem | serde_json::to_value(elem).unwrap())
            .collect::<Vec<_>>();

        if entries.is_empty() {
            return;
        }

        self.segments.push(PathSegment::CommaSeparated {
            entries,
            kind,
        });
    }

    fn path_qualified(
        &mut self,
        self_ty: Ty<'tcx>,
        trait_ref: Option<ty::TraitRef<'tcx>>,
    ) {
        self.pretty_path_qualified(self_ty, trait_ref);
        self.empty_path = false;
    }

    fn pretty_path_qualified(
        &mut self,
        self_ty: Ty<'tcx>,
        trait_ref: Option<ty::TraitRef<'tcx>>,
    ) {
        debug!("pretty_path_qualified {self_ty:?} {trait_ref:?}");
        if trait_ref.is_none() {
            // Inherent impls. Try to print `Foo::bar` for an inherent
            // impl on `Foo`, but fallback to `<Foo>::bar` if self-type is
            // anything other than a simple path.
            match self_ty.kind() {
                ty::Adt(..)
                | ty::Foreign(_)
                | ty::Bool
                | ty::Char
                | ty::Str
                | ty::Int(_)
                | ty::Uint(_)
                | ty::Float(_) => {
                    return self.segments.push(PathSegment::Ty { ty: self_ty });
                }

                _ => {}
            }
        }

        self.generic_delimiters(|cx| {
            cx.segments.push(PathSegment::Impl {
                path: trait_ref.map(|t| TraitRefPrintOnlyTraitPathDefWrapper(t)),
                ty: self_ty,
                kind: ImplKind::As,
            });
        })
    }

    fn path_generic_args(
        &mut self,
        prefix: impl FnOnce(&mut Self),
        args: &[GenericArg<'tcx>],
    ) {
        prefix(self);

        if !args.is_empty() {
            if self.in_value {
                self.segments.push(PathSegment::Colons);
            }
            self.generic_delimiters(|cx| {
                #[derive(Serialize)]
                struct Wrapper<'a, 'tcx: 'a>(#[serde(with = "GenericArgDef")] &'a GenericArg<'tcx>);
                cx.comma_sep(args.into_iter().map(Wrapper), CommaSeparatedKind::GenericArg)
            })
        }
    }
}
