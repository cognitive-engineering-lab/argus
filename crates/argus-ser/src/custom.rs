//! Extensions to the type system for easier consumption.
use rustc_data_structures::fx::FxIndexMap;
use rustc_hir::def_id::DefId;
use rustc_middle::ty;
use serde::Serialize;
#[cfg(feature = "testing")]
use ts_rs::TS;

use crate::ty as myty;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ImplHeader<'tcx> {
  #[serde(with = "myty::Slice__GenericArgDef")]
  #[cfg_attr(feature = "testing", ts(type = "GenericArg[]"))]
  pub args: Vec<ty::GenericArg<'tcx>>,

  #[cfg_attr(feature = "testing", ts(type = "TraitRefPrintOnlyTraitPath"))]
  pub name: crate::TraitRefPrintOnlyTraitPathDef<'tcx>,

  #[serde(with = "myty::TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  pub self_ty: ty::Ty<'tcx>,

  pub predicates: GroupedClauses<'tcx>,

  #[serde(with = "myty::Slice__TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty[]"))]
  pub tys_without_default_bounds: Vec<ty::Ty<'tcx>>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct GroupedClauses<'tcx> {
  pub grouped: Vec<ClauseWithBounds<'tcx>>,
  #[serde(with = "myty::Slice__ClauseDef")]
  #[cfg_attr(feature = "testing", ts(type = "Clause[]"))]
  pub other: Vec<ty::Clause<'tcx>>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ClauseWithBounds<'tcx> {
  #[serde(with = "myty::TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  pub ty: ty::Ty<'tcx>,
  pub bounds: Vec<ClauseBound<'tcx>>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum ClauseBound<'tcx> {
  Trait(
    myty::Polarity,
    #[cfg_attr(feature = "testing", ts(type = "TraitRefPrintOnlyTraitPath"))]
    crate::TraitRefPrintOnlyTraitPathDef<'tcx>,
  ),
  Region(
    #[serde(with = "myty::RegionDef")]
    #[cfg_attr(feature = "testing", ts(type = "Region"))]
    ty::Region<'tcx>,
  ),
}

pub fn group_predicates_by_ty<'tcx>(
  predicates: impl IntoIterator<Item = ty::Clause<'tcx>>,
) -> GroupedClauses<'tcx> {
  // ARGUS: ADDITION: group predicates together based on `self_ty`.
  let mut grouped: FxIndexMap<_, Vec<_>> = FxIndexMap::default();
  let mut other = vec![];
  for p in predicates {
    // TODO: all this binder skipping is a HACK.
    if let Some(poly_trait_pred) = p.as_trait_clause() {
      let ty = poly_trait_pred.self_ty().skip_binder();
      let trait_ref =
        poly_trait_pred.map_bound(|tr| tr.trait_ref).skip_binder();
      let bound = ClauseBound::Trait(
        poly_trait_pred.polarity().into(),
        crate::TraitRefPrintOnlyTraitPathDef(trait_ref),
      );
      grouped.entry(ty).or_default().push(bound);
    } else if let Some(poly_ty_outl) = p.as_type_outlives_clause() {
      let ty = poly_ty_outl.map_bound(|t| t.0).skip_binder();
      let r = poly_ty_outl.map_bound(|t| t.1).skip_binder();
      let bound = ClauseBound::Region(r);
      grouped.entry(ty).or_default().push(bound);
    } else {
      other.push(p);
    }
  }

  let grouped = grouped
    .into_iter()
    .map(|(ty, bounds)| ClauseWithBounds { ty, bounds })
    .collect::<Vec<_>>();

  GroupedClauses { grouped, other }
}

pub fn get_opt_impl_header(
  tcx: ty::TyCtxt,
  def_id: DefId,
) -> Option<ImplHeader> {
  use rustc_data_structures::fx::FxIndexSet;
  let impl_def_id = def_id;

  let trait_ref = tcx.impl_trait_ref(impl_def_id)?.instantiate_identity();
  let args = ty::GenericArgs::identity_for_item(tcx, impl_def_id);

  // FIXME: Currently only handles ?Sized.
  //        Needs to support ?Move and ?DynSized when they are implemented.
  let mut types_without_default_bounds = FxIndexSet::default();
  let sized_trait = tcx.lang_items().sized_trait();

  let arg_names = args
    .iter()
    .filter(|k| k.to_string() != "'_")
    .collect::<Vec<_>>();

  let name = crate::TraitRefPrintOnlyTraitPathDef(trait_ref);
  let self_ty = tcx.type_of(impl_def_id).instantiate_identity();

  // The predicates will contain default bounds like `T: Sized`. We need to
  // remove these bounds, and add `T: ?Sized` to any untouched type parameters.
  let predicates = tcx.predicates_of(impl_def_id).predicates;
  let mut pretty_predicates =
    Vec::with_capacity(predicates.len() + types_without_default_bounds.len());

  for (p, _) in predicates {
    if let Some(poly_trait_ref) = p.as_trait_clause() {
      if Some(poly_trait_ref.def_id()) == sized_trait {
        types_without_default_bounds
          // NOTE: we don't rely on the ordering of the types without bounds here,
          // so `swap_remove` is preferred because it's O(1) instead of `shift_remove`
          // which is O(n).
          .swap_remove(&poly_trait_ref.self_ty().skip_binder());
        continue;
      }
    }
    pretty_predicates.push(*p);
  }

  log::debug!("pretty predicates for impl header {:#?}", pretty_predicates);

  // Argus addition
  let grouped_clauses = group_predicates_by_ty(pretty_predicates);

  let tys_without_default_bounds =
    types_without_default_bounds.into_iter().collect::<Vec<_>>();

  Some(ImplHeader {
    args: arg_names,
    name,
    self_ty,
    predicates: grouped_clauses,
    // predicates: pretty_predicates,
    tys_without_default_bounds,
  })
}
