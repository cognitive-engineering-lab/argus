use rustc_data_structures::fx::FxIndexMap;
use rustc_hir::{
  def_id::DefId,
  definitions::{DefPathDataName, DisambiguatedDefPathData},
  lang_items::LangItem,
};
use rustc_middle::{
  traits::util::supertraits_for_pretty_printing,
  ty::{self, print as pretty_ty, Ty, TyCtxt},
};
use rustc_span::symbol::Symbol;
use rustc_utils::source_map::range::CharRange;
use serde::Serialize;
use smallvec::SmallVec;

use super::{ty as serial_ty, *};

mod default;
mod pretty;

pub struct PathDefNoArgs {
  def_id: DefId,
}

impl PathDefNoArgs {
  pub fn new(def_id: DefId) -> Self {
    Self { def_id }
  }
}

impl Serialize for PathDefNoArgs {
  fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    path_def_no_args(&self.def_id, s)
  }
}

pub(super) fn path_def_no_args<S>(
  def_id: &DefId,
  s: S,
) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  PathBuilder::compile_def_path(*def_id, &[], s)
}

pub struct PathDefWithArgs<'tcx> {
  def_id: DefId,
  args: &'tcx [ty::GenericArg<'tcx>],
}

impl<'tcx> PathDefWithArgs<'tcx> {
  pub fn new(def_id: DefId, args: &'tcx [ty::GenericArg<'tcx>]) -> Self {
    PathDefWithArgs { def_id, args }
  }
}

impl<'tcx> Serialize for PathDefWithArgs<'tcx> {
  fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    PathBuilder::compile_def_path(self.def_id, self.args, s)
  }
}

pub struct AliasPath<'a, 'tcx: 'a> {
  alias_ty: &'a ty::AliasTy<'tcx>,
}

impl<'a, 'tcx: 'a> AliasPath<'a, 'tcx> {
  pub fn new(alias_ty: &'a ty::AliasTy<'tcx>) -> Self {
    AliasPath { alias_ty }
  }
}

impl<'tcx> Serialize for AliasPath<'_, 'tcx> {
  fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    PathBuilder::compile_inherent_projection(self.alias_ty, s)
  }
}

// --------------------------------------------------------
// Value path definitions

pub struct ValuePathWithArgs<'tcx> {
  def_id: DefId,
  args: &'tcx [ty::GenericArg<'tcx>],
}

impl<'tcx> ValuePathWithArgs<'tcx> {
  pub fn new(def_id: DefId, args: &'tcx [ty::GenericArg<'tcx>]) -> Self {
    ValuePathWithArgs { def_id, args }
  }
}

impl<'tcx> Serialize for ValuePathWithArgs<'tcx> {
  fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    PathBuilder::compile_value_path(self.def_id, self.args, s)
  }
}

// --------------------------------------------------------
// Opaque impl types

#[derive(Default)]
pub struct OpaqueFnEntry<'tcx> {
  // The trait ref is already stored as a key, so just track if we have it as a real predicate
  has_fn_once: bool,
  fn_mut_trait_ref: Option<ty::PolyTraitRef<'tcx>>,
  fn_trait_ref: Option<ty::PolyTraitRef<'tcx>>,
  return_ty: Option<ty::Binder<'tcx, ty::Term<'tcx>>>,
}

pub struct OpaqueImplType<'tcx> {
  def_id: DefId,
  args: &'tcx ty::List<ty::GenericArg<'tcx>>,
}

impl<'tcx> OpaqueImplType<'tcx> {
  pub fn new(
    def_id: DefId,
    args: &'tcx ty::List<ty::GenericArg<'tcx>>,
  ) -> Self {
    OpaqueImplType { def_id, args }
  }
}

impl<'tcx> OpaqueImplType<'tcx> {
  fn insert_trait_and_projection(
    &self,
    tcx: TyCtxt<'tcx>,
    trait_ref: ty::PolyTraitRef<'tcx>,
    polarity: ty::ImplPolarity,
    proj_ty: Option<(DefId, ty::Binder<'tcx, ty::Term<'tcx>>)>,
    traits: &mut FxIndexMap<
      (ty::PolyTraitRef<'tcx>, ty::ImplPolarity),
      FxIndexMap<DefId, ty::Binder<'tcx, ty::Term<'tcx>>>,
    >,
    fn_traits: &mut FxIndexMap<ty::PolyTraitRef<'tcx>, OpaqueFnEntry<'tcx>>,
  ) {
    let trait_def_id = trait_ref.def_id();

    // If our trait_ref is FnOnce or any of its children, project it onto the parent FnOnce
    // super-trait ref and record it there.
    // We skip negative Fn* bounds since they can't use parenthetical notation anyway.
    if polarity == ty::ImplPolarity::Positive
      && let Some(fn_once_trait) = tcx.lang_items().fn_once_trait()
    {
      // If we have a FnOnce, then insert it into
      if trait_def_id == fn_once_trait {
        let entry = fn_traits.entry(trait_ref).or_default();
        // Optionally insert the return_ty as well.
        if let Some((_, ty)) = proj_ty {
          entry.return_ty = Some(ty);
        }
        entry.has_fn_once = true;
        return;
      } else if Some(trait_def_id) == tcx.lang_items().fn_mut_trait() {
        let super_trait_ref = supertraits_for_pretty_printing(tcx, trait_ref)
          .find(|super_trait_ref| super_trait_ref.def_id() == fn_once_trait)
          .unwrap();

        fn_traits
          .entry(super_trait_ref)
          .or_default()
          .fn_mut_trait_ref = Some(trait_ref);
        return;
      } else if Some(trait_def_id) == tcx.lang_items().fn_trait() {
        let super_trait_ref = supertraits_for_pretty_printing(tcx, trait_ref)
          .find(|super_trait_ref| super_trait_ref.def_id() == fn_once_trait)
          .unwrap();

        fn_traits.entry(super_trait_ref).or_default().fn_trait_ref =
          Some(trait_ref);
        return;
      }
    }

    // Otherwise, just group our traits and projection types.
    traits
      .entry((trait_ref, polarity))
      .or_default()
      .extend(proj_ty);
  }

  // TODO: what the hell should we do with binders ...
  pub fn wrap_binder<T, O, C: FnOnce(&T, &Self) -> O>(
    &self,
    value: &ty::Binder<'tcx, T>,
    f: C,
  ) -> O
  where
    T: ty::TypeFoldable<TyCtxt<'tcx>>,
  {
    // let old_region_index = self.region_index;
    // let (new_value, _) = self.name_all_regions(value)?;
    let new_value = value.clone().skip_binder();
    let res = f(&new_value, self);
    // self.region_index = old_region_index;
    // self.binder_depth -= 1;
    res
  }
}

impl<'tcx> Serialize for OpaqueImplType<'tcx> {
  fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct OpaqueImpl<'tcx> {
      fn_traits: Vec<FnTrait<'tcx>>,
      traits: Vec<Trait<'tcx>>,
      #[serde(with = "serial_ty::Slice__RegionDef")]
      lifetimes: Vec<ty::Region<'tcx>>,
      has_sized_bound: bool,
      has_negative_sized_bound: bool,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct FnTrait<'tcx> {
      #[serde(with = "serial_ty::Slice__TyDef")]
      params: Vec<Ty<'tcx>>,
      #[serde(with = "serial_ty::Option__TyDef")]
      ret_ty: Option<Ty<'tcx>>,
      kind: FnTraitKind,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Trait<'tcx> {
      #[serde(with = "serial_ty::ImplPolarityDef")]
      polarity: ty::ImplPolarity,
      trait_name: serial_ty::TraitRefPrintOnlyTraitPathDefWrapper<'tcx>,
      #[serde(with = "serial_ty::Slice__GenericArgDef")]
      own_args: &'tcx [ty::GenericArg<'tcx>],
      assoc_args: Vec<AssocItemDef<'tcx>>,
    }

    #[derive(Serialize)]
    struct AssocItemDef<'tcx> {
      #[serde(with = "serial_ty::SymbolDef")]
      name: Symbol,
      #[serde(with = "super::term::TermDef")]
      term: ty::Term<'tcx>,
    }

    #[derive(Serialize)]
    enum FnTraitKind {
      FnMut,
      Fn,
      FnOnce,
    }

    let infcx = get_dynamic_ctx();
    let tcx = infcx.tcx;
    let OpaqueImplType { def_id, args } = self;

    // Grab the "TraitA + TraitB" from `impl TraitA + TraitB`,
    // by looking up the projections associated with the def_id.
    let bounds = tcx.explicit_item_bounds(def_id);

    let mut traits = FxIndexMap::default();
    let mut fn_traits = FxIndexMap::default();
    let mut has_sized_bound = false;
    let mut has_negative_sized_bound = false;
    let mut lifetimes = SmallVec::<[ty::Region<'tcx>; 1]>::new();

    for (predicate, _) in bounds.iter_instantiated_copied(tcx, args) {
      let bound_predicate = predicate.kind();

      match bound_predicate.skip_binder() {
        ty::ClauseKind::Trait(pred) => {
          let trait_ref = bound_predicate.rebind(pred.trait_ref);

          // Don't print `+ Sized`, but rather `+ ?Sized` if absent.
          if Some(trait_ref.def_id()) == tcx.lang_items().sized_trait() {
            match pred.polarity {
              ty::ImplPolarity::Positive | ty::ImplPolarity::Reservation => {
                has_sized_bound = true;
                continue;
              }
              ty::ImplPolarity::Negative => has_negative_sized_bound = true,
            }
          }

          self.insert_trait_and_projection(
            tcx,
            trait_ref,
            pred.polarity,
            None,
            &mut traits,
            &mut fn_traits,
          );
        }
        ty::ClauseKind::Projection(pred) => {
          let proj_ref = bound_predicate.rebind(pred);
          let trait_ref = proj_ref.required_poly_trait_ref(tcx);

          // Projection type entry -- the def-id for naming, and the ty.
          let proj_ty = (proj_ref.projection_def_id(), proj_ref.term());

          self.insert_trait_and_projection(
            tcx,
            trait_ref,
            ty::ImplPolarity::Positive,
            Some(proj_ty),
            &mut traits,
            &mut fn_traits,
          );
        }
        ty::ClauseKind::TypeOutlives(outlives) => {
          lifetimes.push(outlives.1);
        }
        _ => {}
      }
    }

    let mut here_opaque_type = OpaqueImpl {
      fn_traits: vec![],
      traits: vec![],
      lifetimes: vec![],
      has_sized_bound: false,
      has_negative_sized_bound: false,
    };

    for (fn_once_trait_ref, entry) in fn_traits {
      self.wrap_binder(&fn_once_trait_ref, |trait_ref, this| {
        let generics = tcx.generics_of(trait_ref.def_id);
        let own_args = generics.own_args_no_defaults(tcx, trait_ref.args);

        match (entry.return_ty, own_args[0].expect_ty()) {
          (Some(return_ty), arg_tys)
            if matches!(arg_tys.kind(), ty::Tuple(_)) =>
          {
            let kind = if entry.fn_trait_ref.is_some() {
              FnTraitKind::Fn
            } else if entry.fn_mut_trait_ref.is_some() {
              FnTraitKind::FnMut
            } else {
              FnTraitKind::FnOnce
            };

            let params = arg_tys.tuple_fields().iter().collect::<Vec<_>>();
            let ret_ty = return_ty.skip_binder().ty();

            here_opaque_type.fn_traits.push(FnTrait {
              params,
              ret_ty,
              kind,
            });
          }
          // If we got here, we can't print as a `impl Fn(A, B) -> C`. Just record the
          // trait_refs we collected in the OpaqueFnEntry as normal trait refs.
          _ => {
            if entry.has_fn_once {
              traits
                .entry((fn_once_trait_ref, ty::ImplPolarity::Positive))
                .or_default()
                .extend(
                  // Group the return ty with its def id, if we had one.
                  entry.return_ty.map(|ty| {
                    (tcx.require_lang_item(LangItem::FnOnceOutput, None), ty)
                  }),
                );
            }
            if let Some(trait_ref) = entry.fn_mut_trait_ref {
              traits
                .entry((trait_ref, ty::ImplPolarity::Positive))
                .or_default();
            }
            if let Some(trait_ref) = entry.fn_trait_ref {
              traits
                .entry((trait_ref, ty::ImplPolarity::Positive))
                .or_default();
            }
          }
        }
      })
    }

    // Print the rest of the trait types (that aren't Fn* family of traits)
    for ((trait_ref, polarity), assoc_items) in traits {
      self.wrap_binder(&trait_ref, |trait_ref, cx| {
        let trait_name = TraitRefPrintOnlyTraitPathDefWrapper(*trait_ref);

        let generics = tcx.generics_of(trait_ref.def_id);
        let own_args = generics.own_args_no_defaults(tcx, trait_ref.args);
        let mut assoc_args = vec![];

        for (assoc_item_def_id, term) in assoc_items {
          // Skip printing `<{coroutine@} as Coroutine<_>>::Return` from async blocks,
          // unless we can find out what coroutine return type it comes from.
          let term = if let Some(ty) = term.skip_binder().ty()
            && let ty::Alias(ty::Projection, proj) = ty.kind()
            && let Some(assoc) = tcx.opt_associated_item(proj.def_id)
            && assoc.trait_container(tcx) == tcx.lang_items().coroutine_trait()
            && assoc.name == rustc_span::sym::Return
          {
            if let ty::Coroutine(_, args) = args.type_at(0).kind() {
              let return_ty = args.as_coroutine().return_ty();
              if !return_ty.is_ty_var() {
                return_ty.into()
              } else {
                continue;
              }
            } else {
              continue;
            }
          } else {
            term.skip_binder()
          };

          let name = tcx.associated_item(assoc_item_def_id).name;
          assoc_args.push(AssocItemDef { name, term });
        }
      });
    }

    here_opaque_type.has_sized_bound = has_sized_bound;
    here_opaque_type.has_negative_sized_bound = has_negative_sized_bound;
    here_opaque_type.serialize(s)
  }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum PathSegment<'tcx> {
  Colons,     // ::
  LocalCrate, // crate
  RawGuess,   // r#
  DefPathDataName {
    #[serde(with = "serial_ty::SymbolDef")]
    name: Symbol,
    #[serde(skip_serializing_if = "Option::is_none")]
    disambiguator: Option<u32>,
  },
  Crate {
    #[serde(with = "serial_ty::SymbolDef")]
    name: Symbol,
  },
  Ty {
    #[serde(with = "serial_ty::TyDef")]
    ty: Ty<'tcx>,
  },
  GenericDelimiters {
    inner: Vec<PathSegment<'tcx>>,
  }, // < ... >
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
  },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ImplKind {
  As,
  For,
}

impl PathSegment<'_> {
  pub fn unambiguous_name(name: Symbol) -> Self {
    PathSegment::DefPathDataName {
      name: name.clone(),
      disambiguator: None,
    }
  }

  pub fn ambiguous_name(name: Symbol, disambiguator: u32) -> Self {
    PathSegment::DefPathDataName {
      name: name.clone(),
      disambiguator: Some(disambiguator),
    }
  }
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
  pub fn compile_value_path(
    def_id: DefId,
    args: &'tcx [ty::GenericArg<'tcx>],
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self::compile_def_path(def_id, args, s)
  }

  pub fn compile_def_path(
    def_id: DefId,
    args: &'tcx [ty::GenericArg<'tcx>],
    s: S,
  ) -> Result<S::Ok, S::Error>
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

    builder.print_def_path(def_id, args);

    builder.serialize(s)
  }

  pub fn compile_inherent_projection(
    alias_ty: &ty::AliasTy<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
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

    builder.pretty_print_inherent_projection(alias_ty);

    builder.serialize(s)
  }

  fn tcx(&self) -> TyCtxt<'tcx> {
    self.infcx.tcx
  }

  fn serialize(self, s: S) -> Result<S::Ok, S::Error> {
    self.segments.serialize(s)
  }

  fn should_print_verbose(&self) -> bool {
    self.infcx.should_print_verbose()
  }

  pub fn print_value_path(
    &mut self,
    def_id: DefId,
    args: &'tcx [ty::GenericArg<'tcx>],
  ) {
    self.print_def_path(def_id, args)
  }

  pub fn fmt_maybe_verbose(
    &mut self,
    data: &DisambiguatedDefPathData,
    _verbose: bool,
  ) {
    match data.data.name() {
      DefPathDataName::Named(name) => {
        self
          .segments
          .push(PathSegment::ambiguous_name(name, data.disambiguator));
        /* CHANGE: if verbose && data.disambiguator != 0 {
          write!(writer, "{}#{}", name, data.disambiguator)
        } else {
          writer.write_str(name.as_str())
        } */
      }
      DefPathDataName::Anon { namespace } => {
        // CHANGE: write!(writer, "{{{}#{}}}", namespace, data.disambiguator)
        self
          .segments
          .push(PathSegment::ambiguous_name(namespace, data.disambiguator));
      }
    }
  }
}

// pub trait Printer<'tcx>: Sized {
//   fn tcx<'a>(&'a self) -> TyCtxt<'tcx>;

//   fn print_def_path(
//       &mut self,
//       def_id: DefId,
//       args: &'tcx [GenericArg<'tcx>],
//   ) -> Result<(), PrintError> {
//       self.default_print_def_path(def_id, args)
//   }

//   fn print_impl_path(
//       &mut self,
//       impl_def_id: DefId,
//       args: &'tcx [GenericArg<'tcx>],
//       self_ty: Ty<'tcx>,
//       trait_ref: Option<ty::TraitRef<'tcx>>,
//   ) -> Result<(), PrintError> {
//       self.default_print_impl_path(impl_def_id, args, self_ty, trait_ref)
//   }

//   fn print_region(&mut self, region: ty::Region<'tcx>) -> Result<(), PrintError>;

//   fn print_type(&mut self, ty: Ty<'tcx>) -> Result<(), PrintError>;

//   fn print_dyn_existential(
//       &mut self,
//       predicates: &'tcx ty::List<ty::PolyExistentialPredicate<'tcx>>,
//   ) -> Result<(), PrintError>;

//   fn print_const(&mut self, ct: ty::Const<'tcx>) -> Result<(), PrintError>;

//   fn path_crate(&mut self, cnum: CrateNum) -> Result<(), PrintError>;

//   fn path_qualified(
//       &mut self,
//       self_ty: Ty<'tcx>,
//       trait_ref: Option<ty::TraitRef<'tcx>>,
//   ) -> Result<(), PrintError>;

//   fn path_append_impl(
//       &mut self,
//       print_prefix: impl FnOnce(&mut Self) -> Result<(), PrintError>,
//       disambiguated_data: &DisambiguatedDefPathData,
//       self_ty: Ty<'tcx>,
//       trait_ref: Option<ty::TraitRef<'tcx>>,
//   ) -> Result<(), PrintError>;

//   fn path_append(
//       &mut self,
//       print_prefix: impl FnOnce(&mut Self) -> Result<(), PrintError>,
//       disambiguated_data: &DisambiguatedDefPathData,
//   ) -> Result<(), PrintError>;

//   fn path_generic_args(
//       &mut self,
//       print_prefix: impl FnOnce(&mut Self) -> Result<(), PrintError>,
//       args: &[GenericArg<'tcx>],
//   ) -> Result<(), PrintError>;

//   // Defaults (should not be overridden):

//   #[instrument(skip(self), level = "debug")]
//   fn default_print_def_path(
//       &mut self,
//       def_id: DefId,
//       args: &'tcx [GenericArg<'tcx>],
//   ) -> Result<(), PrintError>;

//   fn default_print_impl_path(
//       &mut self,
//       impl_def_id: DefId,
//       _args: &'tcx [GenericArg<'tcx>],
//       self_ty: Ty<'tcx>,
//       impl_trait_ref: Option<ty::TraitRef<'tcx>>,
//   ) -> Result<(), PrintError>;
// }
