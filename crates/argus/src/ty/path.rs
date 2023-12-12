use rustc_type_ir as ir;
use rustc_middle::{ty::{self, *, abstract_const::CastKind}, mir::{BinOp, UnOp}};
use rustc_hir::def_id::{DefId, DefIndex, CrateNum};
use rustc_data_structures::sso::SsoHashSet;
use rustc_span::{symbol::{kw, sym, Ident, Symbol}, Span};
use rustc_target::spec::abi::Abi;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_hir::Unsafety;
use rustc_session::cstore::ExternCrateSource;
// use rustc_middle::ty::print::with_crate_prefix;
use rustc_session::cstore::ExternCrate;
use rustc_hir::definitions::DefPathData;
use rustc_hir::definitions::DisambiguatedDefPathData;
use rustc_hir::def_id::ModDefId;
use rustc_hir::definitions::DefPathDataName;

use serde::{Serialize, ser::SerializeSeq};
use rustc_utils::source_map::range::CharRange;
use super::*;

macro_rules! define_scoped_cx {
    ($cx:ident) => {
        macro_rules! scoped_cx {
            () => {
                $cx
            };
        }
    };
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum PathSegment<'tcx> {
    Crate,
    Colons,
    LAngle,
    RAngle,
    As,
    Symbol(String),
    ImplAt(CharRange),
    Ty(#[serde(with = "TyDef")] Ty<'tcx>),
    TraitOnlyPath(#[serde(with = "TraitRefPrintOnlyTraitPathDef")] TraitRef<'tcx>),
}

struct PathBuilder<'tcx, S: serde::Serializer> {
    tcx: TyCtxt<'tcx>,
    empty_path: bool,
    in_value: bool,
    segments: Vec<PathSegment<'tcx>>,
    _marker: std::marker::PhantomData<S>,
}


pub fn def_path<'tcx, S>(def_id: DefId, args: &'tcx [GenericArg<'tcx>], s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let tcx = get_dynamic_tcx();
    let mut builder = PathBuilder {
        tcx,
        empty_path: true,
        in_value: false,
        segments: Vec::new(),
        _marker: std::marker::PhantomData::<S>,
    };
    builder.def_path(def_id, args)?;
    builder.segments.serialize(s)
}

impl<'tcx, S: serde::Serializer> PathBuilder<'tcx, S> {
    /// Serialize this as a definition path
    /// See [`print_def_path`](https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_middle/ty/print/pretty.rs.html#2608-2610).
    // FIXME: this currently *only* handles a subset of what is necessary,
    // it needs to be made more robust.
    pub fn def_path(&mut self, def_id: DefId, args: &'tcx [GenericArg<'tcx>]) -> Result<(), S::Error>
    {
        if args.is_empty() {
            // TODO:
            // match self.try_print_trimmed_def_path(def_id)? {
            //     true => return Ok(()),
            //     false => {}
            // }

            match self.try_visible_def_path(def_id)? {
                true => return Ok(()),
                false => {}
            }
        }

        let key = self.tcx.def_key(def_id);
        if let DefPathData::Impl = key.disambiguated_data.data {
            // Always use types for non-local impls, where types are always
            // available, and filename/line-number is mostly uninteresting.
            let use_types = !def_id.is_local() || {
                // Otherwise, use filename/line-number if forced.
                // FIXME: let force_no_types = with_forced_impl_filename_line();
                todo!();
                // FIXME: !force_no_types
                true
            };

            if !use_types {
                // If no type info is available, fall back to
                // pretty printing some span information. This should
                // only occur very early in the compiler pipeline.
                let parent_def_id = DefId { index: key.parent.unwrap(), ..def_id };
                let span = self.tcx.def_span(def_id);

                self.def_path(parent_def_id, &[])?;

                // HACK(eddyb) copy of `path_append` to avoid
                // constructing a `DisambiguatedDefPathData`.
                if !self.empty_path {
                    self.segments.push(PathSegment::Colons);
                }

                let source_map = self.tcx.sess.source_map();
                // let span = self.tcx.sess.source_map().span_to_embeddable_string(span);
                let range = CharRange::from_span(span, source_map).unwrap();
                self.segments.push(PathSegment::ImplAt(range));
                // write!(
                //     self,
                //     "<impl at {}>",
                //     // This may end up in stderr diagnostics but it may also be emitted
                //     // into MIR. Hence we use the remapped path if available
                //     self.tcx.sess.source_map().span_to_embeddable_string(span)
                // )?;
                self.empty_path = false;

                return Ok(());
            }
        }

        self.default_def_path(def_id, args)
    }

    fn try_trimmed_def_path(def_id: DefId) -> Result<bool, S::Error> {
        // FIXME: ignoring for now
        todo!()
    }


    fn try_visible_def_path(&mut self, def_id: DefId) -> Result<bool, S::Error> {
        // TODO:
        // if with_no_visible_paths() {
        //     return Ok(false);
        // }

        let mut callers = Vec::new();
        self.try_visible_def_path_recur(def_id, &mut callers)
    }

    fn path_crate(&mut self, cnum: CrateNum) -> Result<(), S::Error> {
        self.empty_path = true;
        let tcx = get_dynamic_tcx();
        if cnum == LOCAL_CRATE {
            if tcx.sess.at_least_rust_2018() {
                // We add the `crate::` keyword on Rust 2018, only when desired.
                if false { // TODO: with_crate_prefix() {
                    self.segments.push(PathSegment::Crate);
                    self.empty_path = false;
                }
            }
        } else {
            let symbol = tcx.crate_name(cnum).to_string();
            self.segments.push(PathSegment::Symbol(symbol));
            self.empty_path = false;
        }
        Ok(())
    }

    /// See [`try_print_visible_def_path_recur`](https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_middle/ty/print/pretty.rs.html#406)
    fn try_visible_def_path_recur(
        &mut self,
        def_id: DefId,
        callers: &mut Vec<DefId>,
    ) -> Result<bool, S::Error> {
        let tcx = self.tcx;

        // If `def_id` is a direct or injected extern crate, return the
        // path to the crate followed by the path to the item within the crate.
        if let Some(cnum) = def_id.as_crate_root() {
            if cnum == LOCAL_CRATE {
                self.path_crate(cnum)?;
                return Ok(true);
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
            match tcx.extern_crate(def_id) {
                Some(&ExternCrate { src, dependency_of, span, .. }) => match (src, dependency_of) {
                    (ExternCrateSource::Extern(def_id), LOCAL_CRATE) => {
                        if span.is_dummy() {
                            self.path_crate(cnum)?;
                            return Ok(true);
                        }

                        // FIXME: (gavinleroy)
                        // with_no_visible_paths!(self.print_def_path(def_id, &[])?);

                        return Ok(true);
                    }
                    (ExternCrateSource::Path, LOCAL_CRATE) => {
                        self.path_crate(cnum)?;
                        return Ok(true);
                    }
                    _ => {}
                },
                None => {
                    self.path_crate(cnum)?;
                    return Ok(true);
                }
            }
        }

        if def_id.is_local() {
            return Ok(false);
        }

        let visible_parent_map = tcx.visible_parent_map(());

        let mut cur_def_key = tcx.def_key(def_id);

        // For a constructor, we want the name of its parent rather than <unnamed>.
        if let DefPathData::Ctor = cur_def_key.disambiguated_data.data {
            let parent = DefId {
                krate: def_id.krate,
                index: cur_def_key
                    .parent
                    .expect("`DefPathData::Ctor` / `VariantData` missing a parent"),
            };

            cur_def_key = tcx.def_key(parent);
        }

        let Some(visible_parent) = visible_parent_map.get(&def_id).cloned() else {
            return Ok(false);
        };

        let actual_parent = tcx.opt_parent(def_id);
        // debug!(
        //     "try_print_visible_def_path: visible_parent={:?} actual_parent={:?}",
        //     visible_parent, actual_parent,
        // );

        let mut data = cur_def_key.disambiguated_data.data;
        // debug!(
        //     "try_print_visible_def_path: data={:?} visible_parent={:?} actual_parent={:?}",
        //     data, visible_parent, actual_parent,
        // );

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
                let reexport = tcx
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
                    return Ok(false);
                }
            }
            // Re-exported `extern crate` (#43189).
            DefPathData::CrateRoot => {
                data = DefPathData::TypeNs(tcx.crate_name(def_id.krate));
            }
            _ => {}
        }
        // debug!("try_print_visible_def_path: data={:?}", data);

        if callers.contains(&visible_parent) {
            return Ok(false);
        }
        callers.push(visible_parent);
        // HACK(eddyb) this bypasses `path_append`'s prefix printing to avoid
        // knowing ahead of time whether the entire path will succeed or not.
        // To support printers that do not implement `PrettyPrinter`, a `Vec` or
        // linked list on the stack would need to be built, before any printing.
        match self.try_visible_def_path_recur(visible_parent, callers)? {
            false => return Ok(false),
            true => {}
        }
        callers.pop();
        self.path_append(|_| Ok(()), &DisambiguatedDefPathData { data, disambiguator: 0 })?;
        Ok(true)
    }

    fn path_append(
        &mut self,
        print_prefix: impl FnOnce(&mut Self) -> Result<(), S::Error>,
        disambiguated_data: &DisambiguatedDefPathData,
    ) -> Result<(), S::Error> {
        // FIXME:
        print_prefix(self)?;

        // Skip `::{{extern}}` blocks and `::{{constructor}}` on tuple/unit structs.
        if let DefPathData::ForeignMod | DefPathData::Ctor = disambiguated_data.data {
            return Ok(());
        }

        let name = disambiguated_data.data.name();
        if !self.empty_path {
            self.segments.push(PathSegment::Colons);
        }

        if let DefPathDataName::Named(name) = name {
            if Ident::with_dummy_span(name).is_raw_guess() {
                todo!()
                // TODO: write!(self, "r#")?;
            }
        }

        // TODO: 
        // let verbose = self.should_print_verbose();
        // disambiguated_data.fmt_maybe_verbose(self, verbose)?;

        self.empty_path = false;

        Ok(())
    }


    fn path_qualified(
        &mut self,
        self_ty: Ty<'tcx>,
        trait_ref: Option<ty::TraitRef<'tcx>>,
    ) -> Result<(), S::Error> {
        self.pretty_path_qualified(self_ty, trait_ref)?;
        self.empty_path = false;
        Ok(())
    }

    fn pretty_path_qualified(
        &mut self,
        self_ty: Ty<'tcx>,
        trait_ref: Option<ty::TraitRef<'tcx>>,
    ) -> Result<(), S::Error> {
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
                    self.segments.push(PathSegment::Ty(self_ty.clone()));
                    return Ok(());
                    // return self_ty.print(self);
                }

                _ => {}
            }
        }

        self.generic_delimiters(|cx| {
            define_scoped_cx!(cx);

            // p!(print(self_ty));
            self.segments.push(PathSegment::Ty(self_ty.clone()));
            if let Some(trait_ref) = trait_ref {
                // p!(" as ", print(trait_ref.print_only_trait_path()));
                self.segments.push(PathSegment::As);
                self.segments.push(PathSegment::TraitOnlyPath(trait_ref.clone()));
            }
            Ok(())
        })
    }

    fn pretty_path_append_impl(
        &mut self,
        print_prefix: impl FnOnce(&mut Self) -> Result<(), S::Error>,
        self_ty: Ty<'tcx>,
        trait_ref: Option<ty::TraitRef<'tcx>>,
    ) -> Result<(), S::Error> {
        print_prefix(self)?;

        self.generic_delimiters(|cx| {
            define_scoped_cx!(cx);

            // FIXME: use serialized wrapper
            // p!("impl ");
            if let Some(trait_ref) = trait_ref {
                // FIXME: use serialized wrapper
                // p!(print(trait_ref.print_only_trait_path()), " for ");
            }
            // p!(print(self_ty));

            Ok(())
        })
    }

    fn path_append_impl(
        &mut self,
        print_prefix: impl FnOnce(&mut Self) -> Result<(), S::Error>,
        _disambiguated_data: &DisambiguatedDefPathData,
        self_ty: Ty<'tcx>,
        trait_ref: Option<ty::TraitRef<'tcx>>,
    ) -> Result<(), S::Error> {
        self.pretty_path_append_impl(
            |cx| {
                print_prefix(cx)?;
                if !cx.empty_path {
                    self.segments.push(PathSegment::Colons);
                }

                Ok(())
            },
            self_ty,
            trait_ref,
        )?;
        self.empty_path = false;
        Ok(())
    }

    fn default_def_path(
        &mut self,
        def_id: DefId,
        args: &'tcx [GenericArg<'tcx>],
    ) -> Result<(), S::Error> {
        let key = self.tcx.def_key(def_id);

        match key.disambiguated_data.data {
            DefPathData::CrateRoot => {
                assert!(key.parent.is_none());
                self.path_crate(def_id.krate)
            }

            DefPathData::Impl => {
                let generics = self.tcx.generics_of(def_id);
                let self_ty = self.tcx.type_of(def_id);
                let impl_trait_ref = self.tcx.impl_trait_ref(def_id);
                let (self_ty, impl_trait_ref) = if args.len() >= generics.count() {
                    (
                        self_ty.instantiate(self.tcx, args),
                        impl_trait_ref.map(|i| i.instantiate(self.tcx, args)),
                    )
                } else {
                    (
                        self_ty.instantiate_identity(),
                        impl_trait_ref.map(|i| i.instantiate_identity()),
                    )
                };
                self.print_impl_path(def_id, args, self_ty, impl_trait_ref)
            }

            _ => {
                let parent_def_id = DefId { index: key.parent.unwrap(), ..def_id };

                let mut parent_args = args;
                let mut trait_qualify_parent = false;
                if !args.is_empty() {
                    let generics = self.tcx.generics_of(def_id);
                    parent_args = &args[..generics.parent_count.min(args.len())];

                    match key.disambiguated_data.data {
                        // Closures' own generics are only captures, don't print them.
                        // TODO: DefPathData::Closure => {}
                        // This covers both `DefKind::AnonConst` and `DefKind::InlineConst`.
                        // Anon consts doesn't have their own generics, and inline consts' own
                        // generics are their inferred types, so don't print them.
                        DefPathData::AnonConst => {}

                        // If we have any generic arguments to print, we do that
                        // on top of the same path, but without its own generics.
                        _ => {
                            if !generics.params.is_empty() && args.len() >= generics.count() {
                                let args = generics.own_args_no_defaults(self.tcx, args);
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
                        && self.tcx.generics_of(parent_def_id).parent_count == 0;
                }

                self.path_append(
                    |cx: &mut Self| {
                        if trait_qualify_parent {
                            let trait_ref = ty::TraitRef::new(
                                cx.tcx,
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

    fn print_impl_path(
        &mut self,
        impl_def_id: DefId,
        args: &'tcx [GenericArg<'tcx>],
        self_ty: Ty<'tcx>,
        trait_ref: Option<ty::TraitRef<'tcx>>,
    ) -> Result<(), S::Error> {
        self.default_print_impl_path(impl_def_id, args, self_ty, trait_ref)
    }

    fn default_print_impl_path(
        &mut self,
        impl_def_id: DefId,
        _args: &'tcx [GenericArg<'tcx>],
        self_ty: Ty<'tcx>,
        impl_trait_ref: Option<ty::TraitRef<'tcx>>,
    ) -> Result<(), S::Error> {
        // debug!(
        //     "default_print_impl_path: impl_def_id={:?}, self_ty={}, impl_trait_ref={:?}",
        //     impl_def_id, self_ty, impl_trait_ref
        // );

        let key = self.tcx.def_key(impl_def_id);
        let parent_def_id = DefId { index: key.parent.unwrap(), ..impl_def_id };

        // Decide whether to print the parent path for the impl.
        // Logically, since impls are global, it's never needed, but
        // users may find it useful. Currently, we omit the parent if
        // the impl is either in the same module as the self-type or
        // as the trait.
        let in_self_mod = match characteristic_def_id_of_type(self_ty) {
            None => false,
            Some(ty_def_id) => self.tcx.parent(ty_def_id) == parent_def_id,
        };
        let in_trait_mod = match impl_trait_ref {
            None => false,
            Some(trait_ref) => self.tcx.parent(trait_ref.def_id) == parent_def_id,
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

    fn path_generic_args(
        &mut self,
        print_prefix: impl FnOnce(&mut Self) -> Result<(), S::Error>,
        args: &[GenericArg<'tcx>],
    ) -> Result<(), S::Error> {
        print_prefix(self)?;

        let tcx = self.tcx;

        let args = args.iter().copied();

        let args: Vec<_> = if !tcx.sess.verbose() {
            // skip host param as those are printed as `~const`
            args.filter(|arg| match arg.unpack() {
                // FIXME(effects) there should be a better way than just matching the name
                GenericArgKind::Const(c)
                    if tcx.features().effects
                        && matches!(
                            c.kind(),
                            ty::ConstKind::Param(ty::ParamConst { name: sym::host, .. })
                        ) =>
                {
                    false
                }
                _ => true,
            })
            .collect()
        } else {
            // If -Zverbose is passed, we should print the host parameter instead
            // of eating it.
            args.collect()
        };

        if !args.is_empty() {
            if self.in_value {
                self.segments.push(PathSegment::Colons);
            }
            self.generic_delimiters(|cx| cx.comma_sep(args.into_iter()))
        } else {
            Ok(())
        }
    }

    fn generic_delimiters(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<(), S::Error>,
    ) -> Result<(), S::Error> {
        self.segments.push(PathSegment::LAngle);

        let was_in_value = std::mem::replace(&mut self.in_value, false);
        f(self)?;
        self.in_value = was_in_value;

        self.segments.push(PathSegment::RAngle);
        Ok(())
    }

    fn comma_sep<T>(&mut self, mut elems: impl Iterator<Item = T>) -> Result<(), S::Error>
    where
        T: Serialize,
    {
        todo!()
        // if let Some(first) = elems.next() {
        //     first.print(self)?;
        //     for elem in elems {
        //         self.write_str(", ")?;
        //         elem.print(self)?;
        //     }
        // }
        // Ok(())
    }
}

fn characteristic_def_id_of_type_cached<'a>(
    ty: Ty<'a>,
    visited: &mut SsoHashSet<Ty<'a>>,
) -> Option<DefId> {
    match *ty.kind() {
        ty::Adt(adt_def, _) => Some(adt_def.did()),

        ty::Dynamic(data, ..) => data.principal_def_id(),

        ty::Array(subty, _) | ty::Slice(subty) => {
            characteristic_def_id_of_type_cached(subty, visited)
        }

        ty::RawPtr(mt) => characteristic_def_id_of_type_cached(mt.ty, visited),

        ty::Ref(_, ty, _) => characteristic_def_id_of_type_cached(ty, visited),

        ty::Tuple(tys) => tys.iter().find_map(|ty| {
            if visited.insert(ty) {
                return characteristic_def_id_of_type_cached(ty, visited);
            }
            return None;
        }),

        ty::FnDef(def_id, _)
        | ty::Closure(def_id, _)
        | ty::Coroutine(def_id, _, _)
        | ty::CoroutineWitness(def_id, _)
        | ty::Foreign(def_id) => Some(def_id),

        ty::Bool
        | ty::Char
        | ty::Int(_)
        | ty::Uint(_)
        | ty::Str
        | ty::FnPtr(_)
        | ty::Alias(..)
        | ty::Placeholder(..)
        | ty::Param(_)
        | ty::Infer(_)
        | ty::Bound(..)
        | ty::Error(_)
        | ty::Never
        | ty::Float(_) => None,
    }
}
pub fn characteristic_def_id_of_type(ty: Ty<'_>) -> Option<DefId> {
    characteristic_def_id_of_type_cached(ty, &mut SsoHashSet::new())
}