use rustc_data_structures::stable_hasher::Hash64;
use rustc_hir::{def_id::DefId, BodyId, HirId};
use rustc_hir_typeck::inspect_typeck;
use rustc_infer::{
  infer::InferCtxt,
  traits::{solve::CandidateSource, ObligationInspector, PredicateObligation},
};
use rustc_middle::ty::{
  self, Predicate, Ty, TyCtxt, TypeSuperVisitable, TypeVisitable, TypeVisitor,
  TypeckResults,
};
use rustc_span::{symbol::sym, FileName, Span};
use rustc_trait_selection::{
  solve::inspect::{InspectCandidate, ProbeKind},
  traits::{
    query::NoSolution,
    solve::{Certainty, MaybeCause},
  },
};
use rustc_utils::source_map::range::CharRange;

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::{hash::StableHash, EvaluationResult};

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

impl<'tcx> TyExt<'tcx> for Ty<'tcx> {
  fn is_error(&self) -> bool {
    matches!(self.kind(), ty::TyKind::Error(..))
  }

  fn is_alias(&self) -> bool {
    matches!(self.kind(), ty::TyKind::Alias(..))
  }

  fn is_local(&self) -> bool {
    match self.kind() {
      ty::TyKind::Ref(_, ty, _) | ty::TyKind::RawPtr(ty, ..) => ty.is_local(),

      ty::TyKind::Adt(def, ..) => def.did().is_local(),

      ty::TyKind::Foreign(def_id)
      | ty::TyKind::FnDef(def_id, ..)
      | ty::TyKind::Closure(def_id, ..)
      | ty::TyKind::CoroutineClosure(def_id, ..)
      | ty::TyKind::Coroutine(def_id, ..)
      | ty::TyKind::CoroutineWitness(def_id, ..) => def_id.is_local(),

      ty::TyKind::Bool
      | ty::TyKind::Tuple(..)
      | ty::TyKind::Char
      | ty::TyKind::Int(..)
      | ty::TyKind::Uint(..)
      | ty::TyKind::Float(..)
      | ty::TyKind::Str
      | ty::TyKind::FnPtr(..)
      | ty::TyKind::Array(..)
      | ty::TyKind::Slice(..)
      | ty::TyKind::Dynamic(..)
      | ty::TyKind::Never
      | ty::TyKind::Alias(..)
      | ty::TyKind::Param(..)
      | ty::TyKind::Bound(..)
      | ty::TyKind::Placeholder(..)
      | ty::TyKind::Pat(..)
      | ty::TyKind::Infer(..)
      | ty::TyKind::Error(..) => false,
    }
  }

  fn base_ty(&self) -> ty::Ty<'tcx> {
    match self.kind() {
      ty::TyKind::Ref(_, ty, _) | ty::TyKind::RawPtr(ty, ..) => ty.base_ty(),
      _ => *self,
    }
  }
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
    let mut local_body_span = hir.body(body_id).value.span;
    while local_body_span.from_expansion() {
      local_body_span = local_body_span.source_callsite();
    }

    span.as_local(local_body_span).unwrap_or(span)
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
    needle: ty::PolyTraitRef<'tcx>,
    haystack: ty::Predicate<'tcx>,
  ) -> bool {
    struct TraitRefVisitor<'tcx> {
      tr: ty::PolyTraitRef<'tcx>,
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
        let my_id = self.tr.def_id();

        if args.is_empty() {
          return false;
        }

        // FIXME: is it always the first type in the args list?
        let proj_ty = args.type_at(0);
        proj_ty == my_ty.skip_binder()
          && self.tcx.is_descendant_of(def_id, my_id)
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
          pp @ ty::ProjectionPredicate {
            projection_term, ..
          },
        )) = predicate.kind().skip_binder()
        {
          use rustc_infer::traits::util::supertraits;

          // Check whether the `TraitRef`, or any implied supertrait
          // appear in the projection. This can happen for example if we have
          // a trait predicate `F: Fn(i32) -> i32`, the projection of the `Output`
          // would be `<F as FnOnce(i32)>::Output == i32`.

          let simple_check = self
            .occurs_in_projection(projection_term.args, projection_term.def_id);
          let deep_check = || {
            let prj_ply_trait_ref = predicate.kind().rebind(pp);
            let poly_supertrait_ref =
              prj_ply_trait_ref.required_poly_trait_ref(self.tcx);
            // Check whether `poly_supertrait_ref` is a supertrait of `self.tr`.
            // HACK FIXME: this is too simplistic, it's unsound to check
            // *just* that the `self_ty`s are equivalent and that the `def_id` is
            // a super trait...
            log::debug!(
              "deep_check:\n  {:?}\n  to super\n  {:?}",
              self.tr,
              poly_supertrait_ref
            );
            for super_ptr in supertraits(self.tcx, self.tr) {
              log::debug!("* against {super_ptr:?}");
              if super_ptr == poly_supertrait_ref {
                return true;
              }
            }
            false
          };

          self.found |= simple_check || deep_check();
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

  fn function_arity(&self, ty: Ty<'tcx>) -> Option<usize> {
    let from_def_id = |did| {
      Some(
        self
          .fn_sig(did)
          .instantiate_identity()
          .inputs()
          .skip_binder()
          .len(),
      )
    };

    let from_sig = |sig: &ty::PolyFnSig| Some(sig.inputs().skip_binder().len());

    match ty.kind() {
      // References to closures are also callable
      ty::TyKind::Ref(_, ty, _) | ty::TyKind::RawPtr(ty, _) => {
        self.function_arity(*ty)
      }
      ty::TyKind::FnDef(def_id, ..) => from_def_id(def_id),
      ty::TyKind::FnPtr(sig) => from_sig(sig),
      ty::TyKind::Closure(_, args) => from_sig(&args.as_closure().sig()),
      ty::TyKind::CoroutineClosure(_, args) => {
        if let ty::TyKind::Tuple(tys) = args
          .as_coroutine_closure()
          .coroutine_closure_sig()
          .skip_binder()
          .tupled_inputs_ty
          .kind()
        {
          Some(tys.len())
        } else {
          None
        }
      }
      _ => None,
    }
  }

  fn fn_trait_arity(&self, t: ty::TraitPredicate<'tcx>) -> Option<usize> {
    let fn_arg_type = t.trait_ref.args.type_at(1);
    if let ty::TyKind::Tuple(tys) = fn_arg_type.kind() {
      Some(tys.len())
    } else {
      None
    }
  }

  fn is_lang_item(&self, def_id: DefId) -> bool {
    self
      .lang_items()
      .iter()
      .any(|(_lang_item, lang_id)| def_id == lang_id)
  }
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

impl<'tcx> PredicateExt<'tcx> for Predicate<'tcx> {
  fn expect_trait_predicate(&self) -> ty::PolyTraitPredicate<'tcx> {
    self.as_trait_predicate().expect("not a trait predicate")
  }

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
    if let Some(tp) = self.as_trait_predicate() {
      let def_id = tp.def_id();
      tcx.is_lang_item(def_id)
    } else {
      false
    }
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

  #[allow(clippy::similar_names)]
  fn is_refined_by(&self, infcx: &InferCtxt<'tcx>, other: &Self) -> bool {
    use std::panic;

    use crate::rustc::InferCtxtExt;
    infcx.probe(move |_| {
      // XXX: The core issue here is that we have a flat list of obligations, and we want
      // to compare them. We only know which `InferCtxt` is associated with a single
      // obligation, not the whole group of obligations that was tried in a single context.
      // Testing is an error implies another means that we can leak inference variables while
      // probing the ENA Tables. Bad. It's safe to catch this panic because we fork the
      // inference contexts and only a single thread has access at a time.
      panic::set_hook(Box::new(|_| {}));
      let res = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let refinee = infcx.freshen(*self);
        let refiner = infcx.freshen(*other);
        infcx.error_implies(refiner, refinee)
      }))
      .unwrap_or(false);
      let _ = panic::take_hook();
      res
    })
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

impl<'tcx, T: TypeVisitable<TyCtxt<'tcx>>> VarCounterExt<'tcx> for T {
  fn count_vars(self, _tcx: TyCtxt<'tcx>) -> usize {
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
