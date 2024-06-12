use rustc_data_structures::{
  fx::FxIndexMap,
  stable_hasher::{Hash64, HashStable, StableHasher},
};
use rustc_hir::{def_id::DefId, BodyId, HirId};
use rustc_hir_typeck::inspect_typeck;
use rustc_infer::{
  infer::InferCtxt,
  traits::{solve::CandidateSource, ObligationInspector, PredicateObligation},
};
use rustc_middle::ty::{
  self, Predicate, Ty, TyCtxt, TypeFoldable, TypeSuperVisitable, TypeVisitable,
  TypeVisitor, TypeckResults,
};
use rustc_query_system::ich::StableHashingContext;
use rustc_span::{symbol::sym, FileName, Span};
use rustc_trait_selection::{
  solve::inspect::{InspectCandidate, ProbeKind},
  traits::{
    query::NoSolution,
    solve::{Certainty, MaybeCause},
  },
};
use rustc_utils::source_map::range::CharRange;
use serde::Serialize;

use crate::{
  analysis::{EvaluationResult, FulfillmentData},
  serialize::{
    self as ser, safe::TraitRefPrintOnlyTraitPathDef,
    ty::PredicateObligationDef,
  },
  types::{
    ClauseBound, ClauseWithBounds, GroupedClauses, ImplHeader, Obligation,
    ObligationNecessity,
  },
};

pub trait CharRangeExt: Copy + Sized {
  /// Returns true if this range touches the `other`.
  fn overlaps(self, other: Self) -> bool;
}

pub trait VarCounterExt<'tcx>: TypeVisitable<TyCtxt<'tcx>> {
  fn count_vars(self, tcx: TyCtxt<'tcx>) -> usize;
}

impl<'tcx, T: TypeVisitable<TyCtxt<'tcx>>> VarCounterExt<'tcx> for T {
  fn count_vars(self, tcx: TyCtxt<'tcx>) -> usize {
    struct TyVarCounterVisitor {
      pub count: usize,
    }

    impl<'tcx> TypeVisitor<TyCtxt<'tcx>> for TyVarCounterVisitor {
      fn visit_ty(&mut self, ty: ty::Ty<'tcx>) {
        if matches!(
          ty.kind(),
          ty::Infer(ty::TyVar(_) | ty::IntVar(_) | ty::FloatVar(_))
        ) {
          self.count += 1;
        }

        ty.super_visit_with(self);
      }

      fn visit_const(&mut self, c: ty::Const<'tcx>) {
        if matches!(c.kind(), ty::ConstKind::Infer(_)) {
          self.count += 1;
        }

        c.super_visit_with(self);
      }

      fn visit_region(&mut self, r: ty::Region<'tcx>) {
        if matches!(r.kind(), ty::RegionKind::ReVar(_)) {
          self.count += 1;
        }
      }
    }

    let mut folder = TyVarCounterVisitor { count: 0 };
    self.visit_with(&mut folder);
    folder.count
  }
}

pub trait PredicateExt<'tcx> {
  fn as_trait_predicate(&self) -> Option<ty::PolyTraitPredicate<'tcx>>;

  fn is_trait_predicate(&self) -> bool;

  fn is_lhs_unit(&self) -> bool;

  fn is_rhs_lang_item(&self, tcx: &TyCtxt) -> bool;

  fn is_trait_pred_rhs(&self, def_id: DefId) -> bool;

  fn is_main_ty_var(&self) -> bool;

  fn is_refined_by(&self, other: &Self) -> bool;
}

impl<'tcx> PredicateExt<'tcx> for Predicate<'tcx> {
  fn as_trait_predicate(&self) -> Option<ty::PolyTraitPredicate<'tcx>> {
    if let ty::PredicateKind::Clause(ty::ClauseKind::Trait(tp)) =
      self.kind().skip_binder()
    {
      Some(self.kind().rebind(tp))
    } else {
      None
    }
  }

  fn is_trait_predicate(&self) -> bool {
    self.as_trait_predicate().is_some()
  }

  fn is_lhs_unit(&self) -> bool {
    matches!(self.kind().skip_binder(),
    ty::PredicateKind::Clause(ty::ClauseKind::Trait(trait_predicate)) if {
      trait_predicate.self_ty().is_unit()
    })
  }

  fn is_rhs_lang_item(&self, tcx: &TyCtxt) -> bool {
    tcx
      .lang_items()
      .iter()
      .any(|(_lang_item, def_id)| self.is_trait_pred_rhs(def_id))
  }

  fn is_trait_pred_rhs(&self, def_id: DefId) -> bool {
    matches!(self.kind().skip_binder(),
    ty::PredicateKind::Clause(ty::ClauseKind::Trait(trait_predicate)) if {
      trait_predicate.def_id() == def_id
    })
  }

  fn is_main_ty_var(&self) -> bool {
    match self.kind().skip_binder() {
      ty::PredicateKind::Clause(ty::ClauseKind::Trait(trait_predicate)) => {
        trait_predicate.self_ty().is_ty_var()
      }
      ty::PredicateKind::Clause(ty::ClauseKind::TypeOutlives(
        ty::OutlivesPredicate(ty, _),
      )) => ty.is_ty_var(),
      ty::PredicateKind::Clause(ty::ClauseKind::Projection(proj)) => {
        proj.self_ty().is_ty_var()
          || proj.term.ty().map_or(false, ty::Ty::is_ty_var)
      }
      _ => false,
    }
  }

  // FIXME: I don't think this is fully correct...but it's been sufficient
  #[allow(clippy::similar_names)]
  fn is_refined_by(&self, other: &Self) -> bool {
    let (Some(refinee), Some(refiner)) =
      (self.as_trait_predicate(), other.as_trait_predicate())
    else {
      return false;
    };

    let refinee = refinee.skip_binder();
    let refiner = refiner.skip_binder();

    // The trait bound must be equal
    if refinee.def_id() != refiner.def_id()
      // The RHS self ty cannot be an inference variable
      || refiner.self_ty().is_ty_var()
      // The impl polarity must be equal
      || refinee.polarity != refiner.polarity
    {
      return false;
    }

    // LHS is _ and RHS is TY with same generic args
    // The LHS self ty must be an inference variable
    (refinee.self_ty().is_ty_var()
      // The generic args must also be the same(...?)
      && refinee.trait_ref.args == refiner.trait_ref.args)
    // LHS TY == RHS TY and generic args were updated...(does this happen?)
    || refinee.self_ty() == refiner.self_ty()
  }
}

pub trait StableHash<'__ctx, 'tcx>:
  HashStable<StableHashingContext<'__ctx>>
{
  fn stable_hash(
    self,
    infcx: &TyCtxt<'tcx>,
    ctx: &mut StableHashingContext<'__ctx>,
  ) -> Hash64;
}

pub trait TyExt<'tcx> {
  fn is_error(&self) -> bool;
}

pub trait TyCtxtExt<'tcx> {
  fn body_filename(&self, body_id: BodyId) -> FileName;

  fn to_local(&self, body_id: BodyId, span: Span) -> Span;

  fn inspect_typeck(
    self,
    body_id: BodyId,
    inspector: ObligationInspector<'tcx>,
  ) -> &TypeckResults;

  fn get_impl_header(&self, def_id: DefId) -> Option<ImplHeader<'tcx>>;

  /// Test whether `a` is a parent node of `b`.
  fn is_parent_of(&self, a: HirId, b: HirId) -> bool;

  fn predicate_hash(&self, p: &Predicate<'tcx>) -> Hash64;

  fn is_annotated_do_not_recommend(
    &self,
    candidate: &InspectCandidate<'_, 'tcx>,
  ) -> bool;

  fn does_trait_ref_occur_in(
    &self,
    needle: ty::TraitRef<'tcx>,
    haystack: ty::Predicate<'tcx>,
  ) -> bool;
}

pub trait TypeckResultsExt<'tcx> {
  fn error_nodes(&self) -> impl Iterator<Item = HirId>;
}

pub trait EvaluationResultExt {
  fn is_yes(&self) -> bool;
  fn is_maybe(&self) -> bool;
  fn is_no(&self) -> bool;
  fn is_better_than(&self, other: &EvaluationResult) -> bool;
  fn yes() -> Self;
  fn no() -> Self;
  fn maybe() -> Self;
}

pub trait PredicateObligationExt {
  fn range(&self, tcx: &TyCtxt, body_id: BodyId) -> CharRange;
}

impl PredicateObligationExt for PredicateObligation<'_> {
  fn range(&self, tcx: &TyCtxt, body_id: BodyId) -> CharRange {
    let source_map = tcx.sess.source_map();
    let hir = tcx.hir();

    let hir_id = hir.body_owner(body_id);
    let body_span = hir.span(hir_id);

    // Backup span of the DefId

    let original_span = self.cause.span;
    let span = if original_span.is_dummy() {
      body_span
    } else {
      original_span
    };

    let span = tcx.to_local(body_id, span);
    CharRange::from_span(span, source_map)
      .expect("failed to get range from span")
  }
}

pub trait InferCtxtExt<'tcx> {
  fn sanitize_obligation(
    &self,
    typeck_results: &'tcx ty::TypeckResults<'tcx>,
    obligation: &PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> PredicateObligation<'tcx>;

  fn bless_fulfilled<'a>(
    &self,
    obligation: &'a PredicateObligation<'tcx>,
    result: EvaluationResult,
    is_synthetic: bool,
  ) -> FulfillmentData<'a, 'tcx>;

  fn erase_non_local_data(
    &self,
    body_id: BodyId,
    fdata: FulfillmentData<'_, 'tcx>,
  ) -> Obligation;

  fn guess_predicate_necessity(
    &self,
    p: &Predicate<'tcx>,
  ) -> ObligationNecessity;

  fn obligation_necessity(
    &self,
    obligation: &PredicateObligation<'tcx>,
  ) -> ObligationNecessity;

  fn predicate_hash(&self, p: &Predicate<'tcx>) -> Hash64;

  fn evaluate_obligation(
    &self,
    obligation: &PredicateObligation<'tcx>,
  ) -> EvaluationResult;
}

// -----------------------------------------------
// Impls

impl CharRangeExt for CharRange {
  fn overlaps(self, other: Self) -> bool {
    self.start < other.end && other.start < self.end
  }
}

impl EvaluationResultExt for EvaluationResult {
  fn is_yes(&self) -> bool {
    matches!(self, EvaluationResult::Ok(Certainty::Yes))
  }

  fn is_maybe(&self) -> bool {
    matches!(self, EvaluationResult::Ok(Certainty::Maybe(..)))
  }

  fn is_no(&self) -> bool {
    matches!(self, EvaluationResult::Err(..))
  }

  fn is_better_than(&self, other: &EvaluationResult) -> bool {
    matches!(
      (self, other),
      (Ok(Certainty::Yes), Ok(Certainty::Maybe(..)))
        | (Ok(Certainty::Maybe(..)), Err(..))
    )
  }

  fn yes() -> Self {
    Ok(Certainty::Yes)
  }

  fn maybe() -> Self {
    Ok(Certainty::Maybe(MaybeCause::Ambiguity))
  }

  fn no() -> Self {
    Err(NoSolution)
  }
}

impl<'__ctx, 'tcx, T> StableHash<'__ctx, 'tcx> for T
where
  T: HashStable<StableHashingContext<'__ctx>>,
  T: TypeFoldable<TyCtxt<'tcx>>,
{
  fn stable_hash(
    self,
    tcx: &TyCtxt<'tcx>,
    ctx: &mut StableHashingContext<'__ctx>,
  ) -> Hash64 {
    let mut h = StableHasher::new();
    let sans_regions = tcx.erase_regions(self);
    sans_regions.hash_stable(ctx, &mut h);
    h.finish()
  }
}

impl<'tcx> TyExt<'tcx> for Ty<'tcx> {
  fn is_error(&self) -> bool {
    matches!(self.kind(), ty::TyKind::Error(..))
  }
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
        TraitRefPrintOnlyTraitPathDef(trait_ref),
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

impl<'tcx> TyCtxtExt<'tcx> for TyCtxt<'tcx> {
  fn body_filename(&self, body_id: BodyId) -> FileName {
    let def_id = body_id.hir_id.owner.to_def_id();
    let span = self.def_span(def_id);
    self.sess.source_map().span_to_filename(span)
  }

  fn to_local(&self, body_id: BodyId, span: Span) -> Span {
    use rustc_utils::source_map::span::SpanExt;
    let hir = self.hir();
    let body_owner = hir.body_owner(body_id);
    let body_span = hir.body(body_id).value.span;
    span.as_local(body_span).unwrap_or(span)
  }

  fn inspect_typeck(
    self,
    body_id: BodyId,
    inspector: ObligationInspector<'tcx>,
  ) -> &TypeckResults {
    let local_def_id = self.hir().body_owner_def_id(body_id);
    // Typeck current body, accumulating inspected information in TLS.
    inspect_typeck(self, local_def_id, inspector)
  }

  fn get_impl_header(&self, def_id: DefId) -> Option<ImplHeader<'tcx>> {
    use rustc_data_structures::fx::FxIndexSet;
    let tcx = *self;
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

    let name = TraitRefPrintOnlyTraitPathDef(trait_ref);
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

  fn is_parent_of(&self, a: HirId, b: HirId) -> bool {
    a == b || self.hir().parent_iter(b).any(|(id, _)| id == a)
  }

  fn predicate_hash(&self, p: &Predicate<'tcx>) -> Hash64 {
    self.with_stable_hashing_context(|mut hcx| p.stable_hash(self, &mut hcx))
  }

  fn is_annotated_do_not_recommend(
    &self,
    candidate: &InspectCandidate<'_, 'tcx>,
  ) -> bool {
    // FIXME: after updating use the function
    // `tcx.has_attrs_with_path` instead of `get_attrs_by_path`
    matches!(candidate.kind(), ProbeKind::TraitCandidate {
            source: CandidateSource::Impl(impl_def_id),
            ..
        } if self.get_attrs_by_path(impl_def_id, &[sym::diagnostic, sym::do_not_recommend]).next().is_some())
  }

  fn does_trait_ref_occur_in(
    &self,
    needle: ty::TraitRef<'tcx>,
    haystack: ty::Predicate<'tcx>,
  ) -> bool {
    struct TraitRefVisitor<'tcx> {
      tr: ty::TraitRef<'tcx>,
      tcx: TyCtxt<'tcx>,
      found: bool,
    }

    impl<'tcx> TraitRefVisitor<'tcx> {
      fn occurs_in_projection(
        &self,
        args: &ty::GenericArgs<'tcx>,
        def_id: DefId,
      ) -> bool {
        let my_ty = self.tr.self_ty();
        let my_id = self.tr.def_id;

        if args.is_empty() {
          return false;
        }

        // FIXME: is it always the first type in the args list?
        let proj_ty = args.type_at(0);
        proj_ty == my_ty && self.tcx.is_descendant_of(def_id, my_id)
      }
    }

    impl<'tcx> TypeVisitor<TyCtxt<'tcx>> for TraitRefVisitor<'tcx> {
      fn visit_ty(&mut self, ty: ty::Ty<'tcx>) {
        log::debug!("*  [{ty:?}]");
        if let ty::TyKind::Alias(ty::AliasTyKind::Projection, alias_ty) =
          ty.kind()
        {
          self.found |=
            self.occurs_in_projection(alias_ty.args, alias_ty.def_id);
        }

        ty.super_visit_with(self);
      }

      fn visit_predicate(&mut self, predicate: ty::Predicate<'tcx>) {
        log::debug!("*  [{predicate:#?}]");

        if let ty::PredicateKind::Clause(ty::ClauseKind::Projection(
          ty::ProjectionPredicate {
            projection_term, ..
          },
        )) = predicate.kind().skip_binder()
        {
          self.found |= self
            .occurs_in_projection(projection_term.args, projection_term.def_id);
        }

        predicate.super_visit_with(self);
      }
    }

    let visitor = &mut TraitRefVisitor {
      tr: needle,
      tcx: *self,
      found: false,
    };

    haystack.visit_with(visitor);

    log::debug!(
      "Checked occurrences {}:\n{needle:#?} ==> {haystack:#?}",
      visitor.found
    );

    visitor.found
  }
}

impl<'tcx> TypeckResultsExt<'tcx> for TypeckResults<'tcx> {
  fn error_nodes(&self) -> impl Iterator<Item = HirId> {
    self
      .node_types()
      .items_in_stable_order()
      .into_iter()
      .filter_map(|(id, ty)| {
        if ty.is_error() {
          Some(HirId {
            owner: self.hir_owner,
            local_id: id,
          })
        } else {
          None
        }
      })
  }
}

impl<'tcx> InferCtxtExt<'tcx> for InferCtxt<'tcx> {
  fn guess_predicate_necessity(
    &self,
    p: &Predicate<'tcx>,
  ) -> ObligationNecessity {
    use ObligationNecessity as ON;

    let is_rhs_lang_item = || {
      self
        .tcx
        .lang_items()
        .iter()
        .any(|(_lang_item, def_id)| p.is_trait_pred_rhs(def_id))
    };

    let is_writeable = || {
      use rustc_type_ir::ClauseKind as CK;
      matches!(
        p.kind().skip_binder(),
        ty::PredicateKind::Clause(
          CK::Trait(..)
            | CK::RegionOutlives(..)
            | CK::TypeOutlives(..)
            | CK::Projection(..)
        )
      )
    };

    if !is_writeable() || p.is_lhs_unit() {
      ON::No
    } else if (p.is_trait_predicate() && is_rhs_lang_item())
      || !p.is_trait_predicate()
    {
      ON::OnError
    } else {
      ON::Yes
    }
  }

  /// Determine what level of "necessity" an obligation has.
  ///
  /// For example, obligations with a cause `SizedReturnType`,
  /// with a `self_ty` `()` (unit), is *unecessary*. Obligations whose
  /// kind is not a Trait Clause, are generally deemed `ForProfessionals`
  /// (that is, you can get them when interested), and others are shown
  /// `OnError`. Necessary obligations are trait predicates where the
  /// type and trait are not `LangItems`.
  fn obligation_necessity(
    &self,
    obligation: &PredicateObligation<'tcx>,
  ) -> ObligationNecessity {
    use rustc_infer::traits::ObligationCauseCode;
    use ObligationNecessity as ON;

    let p = &obligation.predicate;
    let code = obligation.cause.code();

    if matches!(code, ObligationCauseCode::SizedReturnType) && p.is_lhs_unit()
      || matches!(p.as_trait_predicate(), Some(p) if p.self_ty().skip_binder().is_ty_var())
    {
      ON::No
    } else {
      self.guess_predicate_necessity(p)
    }
  }

  fn sanitize_obligation(
    &self,
    typeck_results: &'tcx ty::TypeckResults<'tcx>,
    obligation: &PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> PredicateObligation<'tcx> {
    use crate::rustc::{
      fn_ctx::{FnCtxtExt as RustcFnCtxtExt, FnCtxtSimulator},
      InferCtxtExt as RustcInferCtxtExt,
    };

    match self.to_fulfillment_error(obligation, result) {
      None => obligation.clone(),
      Some(ref mut fe) => {
        let fnctx = FnCtxtSimulator::new(typeck_results, self);
        fnctx.adjust_fulfillment_error_for_expr_obligation(fe);
        fe.obligation.clone()
      }
    }
  }

  fn predicate_hash(&self, p: &Predicate<'tcx>) -> Hash64 {
    let mut freshener = rustc_infer::infer::TypeFreshener::new(self);
    let p = p.fold_with(&mut freshener);
    self.tcx.predicate_hash(&p)
  }

  fn bless_fulfilled<'a>(
    &self,
    obligation: &'a PredicateObligation<'tcx>,
    result: EvaluationResult,
    is_synthetic: bool,
  ) -> FulfillmentData<'a, 'tcx> {
    FulfillmentData {
      hash: self.predicate_hash(&obligation.predicate).into(),
      obligation,
      result,
      is_synthetic,
    }
  }

  fn erase_non_local_data(
    &self,
    body_id: BodyId,
    fdata: FulfillmentData<'_, 'tcx>,
  ) -> Obligation {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Wrapper<'a, 'tcx: 'a>(
      #[serde(with = "PredicateObligationDef")] &'a PredicateObligation<'tcx>,
    );

    let obl = &fdata.obligation;
    let range = obl.range(&self.tcx, body_id);
    let necessity = self.obligation_necessity(obl);
    let obligation = ser::to_value_expect(self, &Wrapper(obl));

    Obligation {
      obligation,
      hash: fdata.hash,
      range,
      kind: fdata.kind(),
      necessity,
      result: fdata.result,
      is_synthetic: fdata.is_synthetic,
    }
  }

  fn evaluate_obligation(
    &self,
    obligation: &PredicateObligation<'tcx>,
  ) -> EvaluationResult {
    use rustc_trait_selection::{
      solve::{GenerateProofTree, InferCtxtEvalExt},
      traits::query::NoSolution,
    };
    match self
      .evaluate_root_goal(obligation.clone().into(), GenerateProofTree::Never)
      .0
    {
      Ok((_, c)) => Ok(c),
      _ => Err(NoSolution),
    }
  }
}
