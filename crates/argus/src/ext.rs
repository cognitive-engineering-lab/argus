use rustc_data_structures::stable_hasher::{Hash64, HashStable, StableHasher};
use rustc_hir::{
  def_id::{DefId, LocalDefId},
  BodyId, HirId, LangItem,
};
use rustc_hir_typeck::inspect_typeck;
use rustc_infer::{
  infer::InferCtxt,
  traits::{ObligationInspector, PredicateObligation},
};
use rustc_middle::ty::{
  self, Predicate, Ty, TyCtxt, TypeFoldable, TypeFolder, TypeSuperFoldable,
  TypeckResults,
};
use rustc_query_system::ich::StableHashingContext;
use rustc_span::Span;
use rustc_utils::source_map::range::CharRange;
use serde::Serialize;

use crate::{
  analysis::{EvaluationResult, FulfillmentData},
  serialize::{serialize_to_value, ty::PredicateDef},
  types::{Obligation, ObligationNecessity},
};

pub trait CharRangeExt: Copy + Sized {
  /// Returns true if this range touches the `other`.
  fn overlaps(self, other: Self) -> bool;
}

pub trait PredicateExt {
  fn is_trait_predicate(&self) -> bool;

  fn is_rhs_lang_item(&self, tcx: &TyCtxt) -> bool;

  fn is_trait_pred_rhs(&self, def_id: DefId) -> bool;
}

impl PredicateExt for Predicate<'_> {
  fn is_trait_predicate(&self) -> bool {
    matches!(
      self.kind().skip_binder(),
      ty::PredicateKind::Clause(ty::ClauseKind::Trait(..))
    )
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
}

pub trait StableHash<'__ctx, 'tcx>:
  HashStable<StableHashingContext<'__ctx>>
{
  fn stable_hash(
    self,
    infcx: &InferCtxt<'tcx>,
    ctx: &mut StableHashingContext<'__ctx>,
  ) -> Hash64;
}

pub trait TyExt<'tcx> {
  fn is_error(&self) -> bool;
}

pub trait TyCtxtExt<'tcx> {
  fn inspect_typeck(
    self,
    body_id: BodyId,
    inspector: ObligationInspector<'tcx>,
  ) -> &TypeckResults;
}

pub trait TypeckResultsExt<'tcx> {
  fn error_nodes(&self) -> impl Iterator<Item = HirId>;
}

pub trait EvaluationResultExt {
  fn is_certain(&self) -> bool;
}

pub trait PredicateObligationExt {
  fn range(&self, tcx: &TyCtxt) -> CharRange;
}

impl PredicateObligationExt for PredicateObligation<'_> {
  fn range(&self, tcx: &TyCtxt) -> CharRange {
    let source_map = tcx.sess.source_map();
    let original_span = Span::source_callsite(self.cause.span);
    CharRange::from_span(original_span, source_map).unwrap_or_else(|_| {
      log::warn!("Scrambling to find range for span {:?}", original_span);
      let def_id = self.cause.body_id.to_def_id();
      let s = tcx
        .hir()
        .span_if_local(def_id)
        .unwrap_or(rustc_span::DUMMY_SP);
      CharRange::from_span(s, source_map)
        .expect("failed to get range from local span")
    })
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
    ldef_id: LocalDefId,
    obligation: &'a PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> FulfillmentData<'a, 'tcx>;

  fn erase_non_local_data(
    &self,
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

  fn body_id(&self) -> Option<LocalDefId>;

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
  fn is_certain(&self) -> bool {
    use rustc_trait_selection::traits::solve::Certainty;
    matches!(self, EvaluationResult::Ok(Certainty::Yes))
  }
}

impl<'__ctx, 'tcx, T> StableHash<'__ctx, 'tcx> for T
where
  T: HashStable<StableHashingContext<'__ctx>>,
  T: TypeFoldable<TyCtxt<'tcx>>,
{
  fn stable_hash(
    self,
    infcx: &InferCtxt<'tcx>,
    ctx: &mut StableHashingContext<'__ctx>,
  ) -> Hash64 {
    let mut h = StableHasher::new();
    let sans_regions = infcx.tcx.erase_regions(self);
    let this =
      sans_regions.fold_with(&mut ty_eraser::TyVarEraserVisitor { infcx });
    // erase infer vars
    this.hash_stable(ctx, &mut h);
    h.finish()
  }
}

impl<'tcx> TyExt<'tcx> for Ty<'tcx> {
  fn is_error(&self) -> bool {
    matches!(self.kind(), ty::TyKind::Error(..))
  }
}

impl<'tcx> TyCtxtExt<'tcx> for TyCtxt<'tcx> {
  fn inspect_typeck(
    self,
    body_id: BodyId,
    inspector: ObligationInspector<'tcx>,
  ) -> &TypeckResults {
    let local_def_id = self.hir().body_owner_def_id(body_id);
    // Typeck current body, accumulating inspected information in TLS.
    inspect_typeck(self, local_def_id, inspector)
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
    use ObligationNecessity::*;

    let is_rhs_lang_item = || {
      self
        .tcx
        .lang_items()
        .iter()
        .any(|(_lang_item, def_id)| p.is_trait_pred_rhs(def_id))
    };

    if !p.is_trait_predicate() {
      ForProfessionals
    } else if is_rhs_lang_item() {
      OnError
    } else {
      Yes
    }
  }

  /// Determine what level of "necessity" an obligation has.
  ///
  /// For example, obligations with a cause `SizedReturnType`,
  /// with a self_ty `()` (unit), is *unecessary*. Obligations whose
  /// kind is not a Trait Clause, are generally deemed `ForProfessionals`
  /// (that is, you can get them when interested), and others are shown
  /// `OnError`. Necessary obligations are trait predicates where the
  /// type and trait are not `LangItems`.
  fn obligation_necessity(
    &self,
    obligation: &PredicateObligation<'tcx>,
  ) -> ObligationNecessity {
    use rustc_infer::traits::ObligationCauseCode::*;
    use ObligationNecessity::*;

    let p = &obligation.predicate;
    let code = obligation.cause.code();

    let is_lhs_unit = || {
      matches!(p.kind().skip_binder(),
      ty::PredicateKind::Clause(ty::ClauseKind::Trait(trait_predicate)) if {
        trait_predicate.self_ty().is_unit()
      })
    };

    if matches!(code, SizedReturnType) && is_lhs_unit() {
      No
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

  // TODO there has to be a better way to do this, right?
  fn body_id(&self) -> Option<LocalDefId> {
    use rustc_infer::traits::DefiningAnchor::*;
    if let Bind(ldef_id) = self.defining_use_anchor {
      Some(ldef_id)
    } else {
      None
    }
  }

  fn predicate_hash(&self, p: &Predicate<'tcx>) -> Hash64 {
    self
      .tcx
      .with_stable_hashing_context(|mut hcx| p.stable_hash(self, &mut hcx))
  }

  fn bless_fulfilled<'a>(
    &self,
    _ldef_id: LocalDefId,
    obligation: &'a PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> FulfillmentData<'a, 'tcx> {
    FulfillmentData {
      hash: self.predicate_hash(&obligation.predicate),
      obligation,
      result,
    }
  }

  fn erase_non_local_data(
    &self,
    fdata: FulfillmentData<'_, 'tcx>,
  ) -> Obligation {
    let obl = &fdata.obligation;
    let range = obl.range(&self.tcx);
    let necessity = self.obligation_necessity(&obl);

    #[derive(Serialize)]
    struct Wrapper<'tcx>(#[serde(with = "PredicateDef")] Predicate<'tcx>);

    let w = Wrapper(obl.predicate.clone());
    let predicate =
      serialize_to_value(self, &w).expect("could not serialize predicate");

    Obligation {
      predicate,
      hash: fdata.hash.into(),
      range,
      kind: fdata.kind(),
      necessity,
      result: fdata.result,
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
      Ok((_, c, _)) => Ok(c),
      _ => Err(NoSolution),
    }
  }
}

mod ty_eraser {
  use super::*;

  pub(super) struct TyVarEraserVisitor<'a, 'tcx: 'a> {
    pub infcx: &'a InferCtxt<'tcx>,
  }

  // FIXME: these placeholders are a huge hack, there's definitely
  // something better we could do here.
  macro_rules! gen_placeholders {
    ($( [$f:ident $n:literal],)*) => {$(
      fn $f(&self) -> Ty<'tcx> {
        Ty::new_placeholder(self.infcx.tcx, ty::PlaceholderType {
          universe: self.infcx.universe(),
          bound: ty::BoundTy {
            var: ty::BoundVar::from_u32(ty::BoundVar::MAX_AS_U32 - $n),
            kind: ty::BoundTyKind::Anon,
          },
        })
      })*
    }
  }

  impl<'a, 'tcx: 'a> TyVarEraserVisitor<'a, 'tcx> {
    gen_placeholders! {
      [ty_placeholder    0],
      [int_placeholder   1],
      [float_placeholder 2],
    }
  }

  impl<'tcx> TypeFolder<TyCtxt<'tcx>> for TyVarEraserVisitor<'_, 'tcx> {
    fn interner(&self) -> TyCtxt<'tcx> {
      self.infcx.tcx
    }

    fn fold_ty(&mut self, ty: Ty<'tcx>) -> Ty<'tcx> {
      // HACK: I'm not sure if replacing type variables with
      // an anonymous placeholder is the best idea. It is *an*
      // idea, certainly. But this should only happen before hashing.
      match ty.kind() {
        ty::Infer(ty::TyVar(_)) => self.ty_placeholder(),
        ty::Infer(ty::IntVar(_)) => self.int_placeholder(),
        ty::Infer(ty::FloatVar(_)) => self.float_placeholder(),
        _ => ty.super_fold_with(self),
      }
    }

    fn fold_binder<T>(&mut self, t: ty::Binder<'tcx, T>) -> ty::Binder<'tcx, T>
    where
      T: TypeFoldable<TyCtxt<'tcx>>,
    {
      let u = self.infcx.tcx.anonymize_bound_vars(t);
      u.super_fold_with(self)
    }
  }
}
