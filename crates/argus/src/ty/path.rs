use rustc_type_ir as ir;
use rustc_middle::{ty::{self, *, abstract_const::CastKind}, mir::{BinOp, UnOp}};
use rustc_hir::def_id::{DefId, DefIndex, CrateNum};
use rustc_data_structures::sso::SsoHashSet;
use rustc_span::{symbol::{kw, sym, Ident, Symbol}, Span};
use rustc_target::spec::abi::Abi;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_hir::Unsafety;
use rustc_session::cstore::ExternCrateSource;
use rustc_middle::ty::print as rustc_print;
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

pub(super) fn path_def_no_args<S>(def_id: DefId, s: S) -> Result<S::Ok, S::Error> 
where
    S: serde::Serializer,
{
    PathBuilder::def_path(def_id, &[], s)
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
        PathBuilder::def_path(self.def_id, self.args, s)
    }
}

// NOTE: this is the type that the PathBuilder
// will build and serialize.
#[derive(Serialize)]
struct DefinedPath {}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum PathSegment<'tcx> {
    Colons,     // ::
    LocalCrate, // crate
    RawGuess,   // r#
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
}

#[derive(Serialize)]
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

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CommaSeparatedKind {
    GenericArg,
}

impl<'a, 'tcx: 'a, S: serde::Serializer> PathBuilder<'a, 'tcx, S> {
    // Used for values instead of definition paths, rustc handles them the same.
    pub fn value_path(def_id: DefId, args: &'tcx [GenericArg<'tcx>], s: S) -> Result<S::Ok, S::Error> 
    where
        S: serde::Serializer,
    {
        Self::def_path(def_id, args, s)
    }

    pub fn def_path(def_id: DefId, args: &'tcx [GenericArg<'tcx>], s: S) -> Result<S::Ok, S::Error>
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
        builder.compile_def_path(def_id, args);
        builder.serialize(s)
    }

    fn tcx(&self) -> TyCtxt<'tcx> {
        self.infcx.tcx
    }

    fn serialize(self, s: S) -> Result<S::Ok, S::Error> {
        self.segments.serialize(s)
    }

    fn try_trimmed_def_path(&mut self, def_id: DefId) -> bool {
        false
    }

    fn try_visible_def_path(&mut self, def_id: DefId) -> bool {
        false
    }

    // Equivalent to rustc_middle::ty::print::pretty::def_path
    // This entry point handles the other grueling cases and handling overflow, 
    // which apparently is a thing in pretty printing! (see issues #55779 and #87932).
    // Equivalent to [rustc_middle::ty::print::pretty::print_def_path].
    fn compile_def_path(&mut self, def_id: DefId, args: &'tcx [GenericArg<'tcx>]) {
        if args.is_empty() {
            if self.try_trimmed_def_path(def_id) {
                return;
            }

            if self.try_visible_def_path(def_id) {
                return;
            }
        }

        // TODO(gavinleroy): rustc does some additional stuff to handle Impl paths,
        // but to my understanding this should only be needed earlier in the compiler
        // pipeline, and isn't necessary for our purposes.

        self.default_def_path(def_id, args)
    }

    // Equivalent to rustc_middle::ty::print::default_def_path
    fn default_def_path(&mut self, def_id: DefId, args: &'tcx [GenericArg<'tcx>]) {
        let key = self.tcx().def_key(def_id);
        log::debug!("DefId {:?} Args {:?} key {:?}", def_id, args, key);

        match key.disambiguated_data.data {
            DefPathData::CrateRoot => {
                assert!(key.parent.is_none());
                self.path_crate(def_id.krate);
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

                self.impl_path(def_id, args, self_ty,  impl_trait_ref);
            }

            _ => {
                let parent_def_id = DefId { index: key.parent.unwrap(), ..def_id };

                let mut parent_args = args;
                let mut trait_qualify_parent = false;
                if !args.is_empty() {
                    log::debug!("Default, with args {:?}", args);
                    let generics = self.tcx().generics_of(def_id);
                    parent_args = &args[..generics.parent_count.min(args.len())];

                    match key.disambiguated_data.data {
                        // Closures' own generics are only captures, don't print them.
                        DefPathData::ClosureExpr => {}
                        // This covers both `DefKind::AnonConst` and `DefKind::InlineConst`.
                        // Anon consts doesn't have their own generics, and inline consts' own
                        // generics are their inferred types, so don't print them.
                        DefPathData::AnonConst => {}

                        // If we have any generic arguments to print, we do that
                        // on top of the same path, but without its own generics.
                        _ => {
                            if !generics.params.is_empty() && args.len() >= generics.count() {
                                let args = generics.own_args_no_defaults(self.tcx(), args);
                                return self.path_generic_args(
                                    |cx| cx.default_def_path(def_id, parent_args),
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

                log::debug!("Trait qualify parent {:?}", trait_qualify_parent);

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
                            cx.default_def_path(parent_def_id, parent_args)
                        }
                    },
                    &key.disambiguated_data,
                )
            }
        }
    }

    fn path_crate(&mut self, cnum: CrateNum) {
        log::debug!("Path crate {:?}", cnum);
        if cnum == LOCAL_CRATE {
            self.segments.push(PathSegment::LocalCrate)
        } else {
            let name = self.tcx().crate_name(cnum);
            self.segments.push(PathSegment::Crate { name })
        }
        self.empty_path = false;
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
        log::debug!(
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
                |cx| cx.default_def_path(parent_def_id, &[]),
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
        self.disambiguated_def_path_data(disambiguated_data);
        self.empty_path = false;
    }

    fn disambiguated_def_path_data(&mut self, disambiguated_data: &DisambiguatedDefPathData) {
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
            define_scoped_cx!(cx);
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
            define_scoped_cx!(cx);
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

        let tcx = self.tcx();

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
            self.generic_delimiters(|cx| {
                #[derive(Serialize)]
                struct Wrapper<'tcx>(#[serde(with = "GenericArgDef")] GenericArg<'tcx>);
                cx.comma_sep(args.into_iter().map(Wrapper), CommaSeparatedKind::GenericArg)
            })
        }
    }
}
