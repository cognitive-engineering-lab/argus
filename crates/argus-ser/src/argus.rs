//! Extensions to the type system for easier consumption.
use std::path::PathBuf;

use argus_ext::ty::TyCtxtExt;
use itertools::Itertools;
use rustc_data_structures::fx::FxIndexMap;
use rustc_hir::def_id::DefId;
use rustc_macros::TypeVisitable;
use rustc_middle::ty::{self, Upcast};
use rustc_utils::source_map::range::CharRange;
use serde::Serialize;
use smallvec::SmallVec;
#[cfg(feature = "testing")]
use ts_rs::TS;

use crate::ty as myty;

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
/// A `DefLocation` definition equivalent to that provided by VSCode's LSP.
pub struct DefLocation {
  r: CharRange,
  f: PathBuf,
}

impl DefLocation {
  pub fn from_def_id_tcx(def_id: DefId, tcx: ty::TyCtxt) -> Option<Self> {
    use rustc_span::{FileName, RealFileName};

    let span = tcx.def_span(def_id);
    let source_map = tcx.sess.source_map();
    let r = CharRange::from_span(span, source_map).ok()?;
    let f = match &source_map.lookup_source_file(span.lo()).name {
      FileName::Real(RealFileName::LocalPath(filename))
      | FileName::Real(RealFileName::Remapped {
        local_path: Some(filename),
        ..
      }) => filename.clone(),
      _ => return None,
    };

    Some(Self { r, f })
  }

  /// NOTE: this must be called within the dynamic context of
  /// the `argus-ser` crate, do not call this directly.
  pub(crate) fn from_def_id(def_id: DefId) -> Option<Self> {
    use rustc_infer::infer::InferCtxt;

    use super::DynCtxt;

    InferCtxt::access(|infcx| {
      let tcx = infcx.tcx;
      Self::from_def_id_tcx(def_id, tcx)
    })
  }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ImplHeader<'tcx> {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub l: Option<DefLocation>,

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

#[derive(Debug, Clone, TypeVisitable, Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct GroupedClauses<'tcx> {
  #[serde(with = "Slice__PolyClauseWithBoundsDef")]
  #[cfg_attr(feature = "testing", ts(type = "PolyClauseWithBounds[]"))]
  pub grouped: Vec<PolyClauseWithBounds<'tcx>>,

  #[serde(with = "myty::Slice__ClauseDef")]
  #[cfg_attr(feature = "testing", ts(type = "Clause[]"))]
  pub other: Vec<ty::Clause<'tcx>>,
}

pub struct Slice__PolyClauseWithBoundsDef;
impl Slice__PolyClauseWithBoundsDef {
  pub fn serialize<S>(
    value: &[PolyClauseWithBounds],
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    #[derive(Serialize)]
    struct Wrapper<'a, 'tcx: 'a>(
      #[serde(with = "Binder__ClauseWithBounds")]
      &'a PolyClauseWithBounds<'tcx>,
    );

    crate::serialize_custom_seq! { Wrapper, s, value }
  }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export, rename = "PolyClauseWithBounds"))]
pub struct Binder__ClauseWithBounds<'tcx> {
  value: ClauseWithBounds<'tcx>,

  #[serde(with = "myty::Slice__BoundVariableKindDef")]
  #[cfg_attr(feature = "testing", ts(type = "BoundVariableKind[]"))]
  bound_vars: &'tcx ty::List<ty::BoundVariableKind>,
}

impl<'tcx> Binder__ClauseWithBounds<'tcx> {
  pub fn new(value: &PolyClauseWithBounds<'tcx>) -> Self {
    Self {
      bound_vars: value.bound_vars(),
      value: value.clone().skip_binder(),
    }
  }

  pub fn serialize<S>(
    value: &PolyClauseWithBounds<'tcx>,
    s: S,
  ) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    Self::new(value).serialize(s)
  }
}

type PolyClauseWithBounds<'tcx> = ty::Binder<'tcx, ClauseWithBounds<'tcx>>;

#[derive(Debug, Clone, TypeVisitable, Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub struct ClauseWithBounds<'tcx> {
  #[serde(with = "myty::TyDef")]
  #[cfg_attr(feature = "testing", ts(type = "Ty"))]
  pub ty: ty::Ty<'tcx>,
  pub bounds: Vec<ClauseBound<'tcx>>,
}

#[derive(Debug, Copy, Clone, TypeVisitable, Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
pub enum ClauseBound<'tcx> {
  Trait(
    myty::Polarity,
    #[serde(with = "myty::TraitRefPrintOnlyTraitPathDef")]
    #[cfg_attr(feature = "testing", ts(type = "TraitRefPrintOnlyTraitPath"))]
    ty::TraitRef<'tcx>,
  ),
  FnTrait(
    myty::Polarity,
    #[serde(with = "myty::TraitRefPrintOnlyTraitPathDef")]
    #[cfg_attr(feature = "testing", ts(type = "TraitRefPrintOnlyTraitPath"))]
    ty::TraitRef<'tcx>,
    #[serde(with = "myty::TyDef")]
    #[cfg_attr(feature = "testing", ts(type = "Ty"))]
    ty::Ty<'tcx>,
  ),
  Region(
    #[serde(with = "myty::RegionDef")]
    #[cfg_attr(feature = "testing", ts(type = "Region"))]
    ty::Region<'tcx>,
  ),
}

pub(crate) fn group_predicates_by_ty<'tcx>(
  tcx: ty::TyCtxt<'tcx>,
  predicates: impl IntoIterator<Item = ty::Clause<'tcx>>,
) -> GroupedClauses<'tcx> {
  // ARGUS: ADDITION: group predicates together based on `self_ty`.
  let mut grouped = FxIndexMap::<_, Vec<_>>::default();
  let mut other = vec![];

  // TODO: this only looks at the output of an `FnOnce`, we probably also need
  // to handle `AsyncFnOnce` and consider doing the same for the output of
  // a `Future`. A further goal could be sugaring all associated type bounds
  // back into the signature but that would require more work (not sure how much).
  let fn_trait_output = tcx.lang_items().fn_once_output();
  let mut fn_output_projections = vec![];

  for p in predicates {
    if let Some(poly_trait_pred) = p.as_trait_clause() {
      let ty = poly_trait_pred.self_ty().skip_binder();
      let trait_ref =
        poly_trait_pred.map_bound(|tr| tr.trait_ref).skip_binder();
      let bound =
        ClauseBound::Trait(poly_trait_pred.polarity().into(), trait_ref);
      grouped
        .entry(ty)
        .or_default()
        .push(poly_trait_pred.rebind(bound));
      continue;
    }

    if let Some(poly_ty_outl) = p.as_type_outlives_clause() {
      let ty = poly_ty_outl.map_bound(|t| t.0).skip_binder();
      let r = poly_ty_outl.map_bound(|t| t.1).skip_binder();
      let bound = ClauseBound::Region(r);
      grouped
        .entry(ty)
        .or_default()
        .push(poly_ty_outl.rebind(bound));
      continue;
    }

    if let Some(poly_projection) = p.as_projection_clause() {
      if let Some(output_defid) = fn_trait_output {
        if poly_projection.item_def_id() == output_defid {
          fn_output_projections.push(poly_projection);
          continue;
        }
      }
    }

    other.push(p);
  }

  let grouped = grouped
    .into_iter()
    .map(|(ty, bounds)| {
      // NOTE: we have to call unique to make a `List`
      let all_bound_vars = bounds.iter().flat_map(|b| b.bound_vars()).unique();
      let bound_vars = tcx.mk_bound_variable_kinds_from_iter(all_bound_vars);
      let unbounds = bounds
        .into_iter()
        .map(|bclause| {
          let clause = bclause.skip_binder();
          if let ClauseBound::Trait(p, tref) = clause {
            if tcx.is_fn_trait(tref.def_id) {
              let poly_tr = bclause.rebind(tref);

              let mut to_remove = SmallVec::<[_; 4]>::new();
              let mut matching_projection = None;

              for (i, p) in fn_output_projections.iter().enumerate() {
               if  tcx.does_trait_ref_occur_in(
                    poly_tr,
                    p.map_bound(|p| {
                      ty::PredicateKind::Clause(ty::ClauseKind::Projection(p))
                    })
                     .upcast(tcx),
                  ) {
                 log::debug!("Removing matching projection {p:#?}");
                 to_remove.push(i);
               }
              }

              while let Some(i) = to_remove.pop() {
                matching_projection = Some(fn_output_projections.remove(i));
              }

              if let Some(proj) = matching_projection {
                log::debug!(
                  "Matching projections for {bclause:?} {matching_projection:#?}"
                );
                let ret_ty = proj
                  .term()
                  .skip_binder()
                  .as_type()
                  .expect("FnOnce::Output Ty");

                return ClauseBound::FnTrait(p, tref, ret_ty);
              }
            }
          }

          clause
        })
        .collect();
      ty::Binder::bind_with_vars(
        ClauseWithBounds {
          ty,
          bounds: unbounds,
        },
        bound_vars,
      )
    })
    .collect::<Vec<_>>();

  assert!(
    fn_output_projections.is_empty(),
    "Remaining output projections {fn_output_projections:#?}"
  );

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
  let grouped_clauses = group_predicates_by_ty(tcx, pretty_predicates);

  let tys_without_default_bounds =
    types_without_default_bounds.into_iter().collect::<Vec<_>>();

  // XXX: This function is not in the dynamic context, we have to
  // pass the `TyCtxt` explicitly.
  let l = DefLocation::from_def_id_tcx(impl_def_id, tcx);

  Some(ImplHeader {
    l,
    args: arg_names,
    name,
    self_ty,
    predicates: grouped_clauses,
    tys_without_default_bounds,
  })
}
