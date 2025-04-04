//! Default implementations from `rustc_middle::ty::print`

use rustc_data_structures::sso::SsoHashSet;
use rustc_hir::{def_id::DefId, definitions::DefPathData};
use rustc_middle::ty::{self, *};

use super::*;

pub trait PathBuilderDefault<'tcx> {
  fn default_print_def_path(
    &mut self,
    def_id: DefId,
    args: &'tcx [GenericArg<'tcx>],
  );

  fn print_impl_path(
    &mut self,
    impl_def_id: DefId,
    args: &'tcx [GenericArg<'tcx>],
    self_ty: Ty<'tcx>,
    trait_ref: Option<ty::TraitRef<'tcx>>,
  );

  fn default_print_impl_path(
    &mut self,
    impl_def_id: DefId,
    _args: &'tcx [GenericArg<'tcx>],
    self_ty: Ty<'tcx>,
    impl_trait_ref: Option<ty::TraitRef<'tcx>>,
  );
}

impl<'tcx> PathBuilderDefault<'tcx> for PathBuilder<'tcx> {
  fn default_print_def_path(
    &mut self,
    def_id: DefId,
    args: &'tcx [GenericArg<'tcx>],
  ) {
    let key = self.tcx().def_key(def_id);
    log::trace!("default_print_def_path {:?}", key);

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
        self.print_impl_path(def_id, args, self_ty, impl_trait_ref);
      }

      _ => {
        let parent_def_id = DefId {
          index: key.parent.unwrap(),
          ..def_id
        };

        let mut parent_args = args;
        let mut trait_qualify_parent = false;
        if !args.is_empty() {
          let generics = self.tcx().generics_of(def_id);
          parent_args = &args[.. generics.parent_count.min(args.len())];

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
              if !generics.own_params.is_empty()
                && args.len() >= generics.count()
              {
                let args = generics.own_args_no_defaults(self.tcx(), args);
                return self.path_generic_args(
                  |cx| cx.print_def_path(def_id, parent_args),
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
              cx.path_qualified(trait_ref.self_ty(), Some(trait_ref));
            } else {
              cx.print_def_path(parent_def_id, parent_args);
            }
          },
          &key.disambiguated_data,
        );
      }
    };
  }

  fn print_impl_path(
    &mut self,
    impl_def_id: DefId,
    args: &'tcx [GenericArg<'tcx>],
    self_ty: Ty<'tcx>,
    trait_ref: Option<ty::TraitRef<'tcx>>,
  ) {
    self.default_print_impl_path(impl_def_id, args, self_ty, trait_ref);
  }

  fn default_print_impl_path(
    &mut self,
    impl_def_id: DefId,
    _args: &'tcx [GenericArg<'tcx>],
    self_ty: Ty<'tcx>,
    impl_trait_ref: Option<ty::TraitRef<'tcx>>,
  ) {
    log::trace!(
        "default_print_impl_path: impl_def_id={:?}, self_ty={}, impl_trait_ref={:?}",
        impl_def_id, self_ty, impl_trait_ref
    );

    let key = self.tcx().def_key(impl_def_id);
    let parent_def_id = DefId {
      index: key.parent.unwrap(),
      ..impl_def_id
    };

    // Decide whether to print the parent path for the impl.
    // Logically, since impls are global, it's never needed, but
    // users may find it useful. Currently, we omit the parent if
    // the impl is either in the same module as the self-type or
    // as the trait.
    let in_self_mod = match characteristic_def_id_of_type(self_ty) {
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
        |cx| cx.print_def_path(parent_def_id, &[]),
        &key.disambiguated_data,
        self_ty,
        impl_trait_ref,
      );
    } else {
      // Otherwise, try to give a good form that would be valid language
      // syntax. Preferably using associated item notation.
      self.path_qualified(self_ty, impl_trait_ref);
    }
  }
}

/// As a heuristic, when we see an impl, if we see that the
/// 'self type' is a type defined in the same module as the impl,
/// we can omit including the path to the impl itself. This
/// function tries to find a "characteristic `DefId`" for a
/// type. It's just a heuristic so it makes some questionable
/// decisions and we may want to adjust it later.
///
/// Visited set is needed to avoid full iteration over
/// deeply nested tuples that have no `DefId`.
fn characteristic_def_id_of_type_cached<'a>(
  ty: Ty<'a>,
  visited: &mut SsoHashSet<Ty<'a>>,
) -> Option<DefId> {
  match *ty.kind() {
    ty::Adt(adt_def, _) => Some(adt_def.did()),

    ty::Dynamic(data, ..) => data.principal_def_id(),

    ty::Pat(subty, _) | ty::Array(subty, _) | ty::Slice(subty) => {
      characteristic_def_id_of_type_cached(subty, visited)
    }

    ty::RawPtr(ty, _) => characteristic_def_id_of_type_cached(ty, visited),

    ty::Ref(_, ty, _) => characteristic_def_id_of_type_cached(ty, visited),

    ty::Tuple(tys) => tys.iter().find_map(|ty| {
      if visited.insert(ty) {
        return characteristic_def_id_of_type_cached(ty, visited);
      }
      None
    }),

    ty::FnDef(def_id, _)
    | ty::Closure(def_id, _)
    | ty::CoroutineClosure(def_id, _)
    | ty::Coroutine(def_id, _)
    | ty::CoroutineWitness(def_id, _)
    | ty::Foreign(def_id) => Some(def_id),

    ty::Bool
    | ty::Char
    | ty::Int(..)
    | ty::Uint(..)
    | ty::Str
    | ty::FnPtr(..)
    | ty::Alias(..)
    | ty::Placeholder(..)
    | ty::UnsafeBinder(..)
    | ty::Param(..)
    | ty::Infer(..)
    | ty::Bound(..)
    | ty::Error(..)
    | ty::Never
    | ty::Float(..) => None,
  }
}

pub fn characteristic_def_id_of_type(ty: Ty<'_>) -> Option<DefId> {
  characteristic_def_id_of_type_cached(ty, &mut SsoHashSet::new())
}
