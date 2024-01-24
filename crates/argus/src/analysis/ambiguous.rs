//! Analysis for ambiguous method calls.
//!
//! This file "simulates" what `rustc_hir_typeck` does for type-checking
//! method call expressions. Only, that we want to keep around *a lot*
//! more information.
use rustc_hir::{self as hir, HirId, LangItem};
use rustc_middle::{
  traits::query::CandidateStep,
  ty::{
    ClauseKind, Predicate, PredicateKind, TraitPredicate, Ty, TyCtxt,
    TypeckResults,
  },
};
use rustc_span::Span;
use rustc_utils::source_map::range::CharRange;

use super::tls::FullObligationData;
use crate::{
  analysis::{entry::ErrorAssemblyCtx, Provenance},
  ext::{TyCtxtExt, TypeckResultsExt},
  serialize::serialize_to_value,
  types::{
    AmbiguityError, MethodLookup, MethodStep, ObligationHash, ReceiverAdjStep,
  },
};

/// Comprehensive data for an ambiguous method call: `obj.frobnicate(a1, a2, ...)`
#[derive(Debug)]
struct AmbigMethodCallExpr<'tcx> {
  /// The entire method call expression, `obj.frobnicate(a1, a2, ...)`.
  pub expr: &'tcx hir::Expr<'tcx>,

  /// Expr of the receiver `obj`.
  pub recvr: &'tcx hir::Expr<'tcx>,

  /// Method call segment, `frobnicate`.
  pub segment: &'tcx hir::PathSegment<'tcx>,

  /// Method call arguments, `a1, a2, ...`.
  pub args: &'tcx [hir::Expr<'tcx>],

  pub call_span: Span,
}

// NOTE: we cannot use the `TypeckResults::is_method_call` because it
// check the `type_dependent_defs` table which *doesn't have* an entry
// for unresolved method calls.
fn get_ambiguous_trait_method_exprs<'tcx>(
  tcx: &TyCtxt<'tcx>,
  typeck_results: &TypeckResults<'tcx>,
) -> Vec<AmbigMethodCallExpr<'tcx>> {
  let hir = tcx.hir();

  log::debug!(
    "Searching for error nodes:\n\t{:#?}",
    typeck_results.error_nodes().collect::<Vec<_>>()
  );

  typeck_results
    .error_nodes()
    .filter_map(|hir_id| {
      let expr = hir.expect_expr(hir_id);

      let hir::ExprKind::MethodCall(segment, recvr, args, span) = &expr.kind
      else {
        return None;
      };

      Some(AmbigMethodCallExpr {
        expr,
        recvr,
        segment,
        args,
        call_span: *span,
      })
    })
    .collect::<Vec<_>>()
}

impl<'a, 'tcx: 'a> ErrorAssemblyCtx<'a, 'tcx> {
  pub fn assemble_ambiguous_errors(&mut self) -> Vec<AmbiguityError> {
    let ambiguous_method_calls =
      get_ambiguous_trait_method_exprs(&self.tcx, self.typeck_results);

    log::debug!("Found {} ambiguous call(s)", ambiguous_method_calls.len());

    let mut errors = vec![];
    for method_call in ambiguous_method_calls {
      if let Some(err) = self.build_ambiguous_error(&method_call) {
        errors.push(err);
      }
    }

    errors
  }

  // When is an obligation associated with an ambiguous expression.
  //
  // Take as an example the ambiguous expression: `obj.frobnicate(a1, a2)`.
  //
  // 1. Get the autoderef steps for obj.
  //
  // 2. For each step S_i, drain the associated queries for:
  //    `S_i: Deref`
  //    (iff successful, then) `AliasRelate( [S_i], Deref::Target, ?tyvar ) S_i: Deref`
  //
  // Quick side NOTE: each of these steps seems to produce a `TY: Sized` query, for now,
  // just filter these out. Later, we will want to show these associated at each step
  // because it is what determines if a type may need to be autoref'ed or not.
  //
  // 3. For each step S_i drain all obligations of the form `TraitPredicate(`S_i: TRAIT`)`.
  //
  // 4. Sort each level by a determined trait order.
  //    (I.e., how rustc looks for them when doing an `AllTraits` query.)
  fn build_ambiguous_error(
    &mut self,
    data: &AmbigMethodCallExpr<'tcx>,
  ) -> Option<AmbiguityError> {
    use rustc_infer::infer::canonical::OriginalQueryValues;
    use rustc_middle::ty::ParamEnvAnd;

    let mut assoc_obls = self
      .obligations
      .extract_if(|prov| prov.child_of(&self.tcx, data.expr.hir_id))
      .collect::<Vec<_>>();

    let peelopt_to_pred = |prov: Option<&Provenance<ObligationHash>>| {
      prov.map(|prov| {
        self
          .obligation_data
          .get(prov.full_data.unwrap())
          .obligation
          .predicate
      })
    };

    let peel_to_pred = |provs: &[Provenance<ObligationHash>]| {
      provs
        .iter()
        .map(|prov| peelopt_to_pred(Some(prov)))
        .collect::<Vec<_>>()
    };

    log::debug!("REMAINING OBLS {:#?}", peel_to_pred(&self.obligations));
    log::debug!("ASSOCIATED OBLIGATIONS {:#?}", peel_to_pred(&assoc_obls));

    // FIXME: horrible printf debugging, we can do better!
    if assoc_obls.is_empty() {
      log::error!(
        r#"expected associated obligations at {:?}
              ambiguous method call: {:#?}
              remaining obligations: {:#?}
        "#,
        data.call_span,
        self.tcx.hir().node_to_string(data.expr.hir_id),
        peel_to_pred(&self.obligations),
      );
      return None;
    }

    let ty_0 = self.typeck_results.expr_ty(data.recvr);

    // FIXME: using a "random" infcx will be bad ™
    let obl_idx = assoc_obls.iter().find_map(|p| p.full_data)?;
    let full_data = self.obligation_data.get(obl_idx);
    let param_env = full_data.obligation.param_env;
    let infcx = &full_data.infcx;

    // Get the autoderef steps of the monomporphized receiver type, T_0.
    //
    let mut orig_values = OriginalQueryValues::default();
    let param_env_and_self_ty = infcx.canonicalize_query(
      ParamEnvAnd {
        param_env,
        value: ty_0,
      },
      &mut orig_values,
    );

    let steps = self.tcx.method_autoderef_steps(param_env_and_self_ty);

    // TODO: handle recursion overflow limit
    // TODO: handle bad receiver type

    // NOTE: I'm skipping inherent candidates as that's not interesting for trait errors.

    let derefs = steps
      .steps
      .iter()
      .map(|step| {
        let mut inner = assoc_obls
          .extract_if(|prov| {
            let Some(idx) = prov.full_data else {
              return false;
            };

            let obldata = self.obligation_data.get(idx);
            self.is_deref_of(step, obldata)
          })
          .collect::<Vec<_>>();
        assert!(inner.len() <= 1);
        inner.pop()
      })
      .collect::<Vec<_>>();

    let alias_relates = steps
      .steps
      .iter()
      .zip(derefs.iter())
      .map(|(step, deref_queries)| {
        let mut inner = assoc_obls
          .extract_if(|prov| {
            let Some(idx) = prov.full_data else {
              return false;
            };

            let obldata = self.obligation_data.get(idx);
            self.is_trait_alias_for(step, deref_queries, obldata)
          })
          .collect::<Vec<_>>();
        assert!(inner.len() <= 1);
        inner.pop()
      })
      .collect::<Vec<_>>();

    let ty_queries = steps
      .steps
      .iter()
      .map(|step| {
        assoc_obls
          .extract_if(|prov| {
            let Some(idx) = prov.full_data else {
              return false;
            };

            let obldata = self.obligation_data.get(idx);
            self.is_trait_predicate_for(step, obldata)
          })
          .collect::<Vec<_>>()
      })
      .collect::<Vec<_>>();

    let mut table = vec![];
    for (step, deref, alias_relate, preds) in
      itertools::izip!(steps.steps, derefs, alias_relates, ty_queries)
    {
      log::debug!(
        r#"At the given STEP {:?}
               DEREFS: {:?}
               ALIAS_RELATE: {:?}
               PRED: {:?}
        "#,
        step.step_ty(),
        peelopt_to_pred(deref.as_ref()),
        peelopt_to_pred(alias_relate.as_ref()),
        peel_to_pred(&preds)
      );

      table.push(MethodStep {
        step: ReceiverAdjStep { ty: step.step_ty() },
        deref_query: deref.map(|f| f.forget()),
        relate_query: alias_relate.map(|f| f.forget()),
        trait_predicates: preds
          .into_iter()
          .map(|f| f.forget())
          .collect::<Vec<_>>(),
      });
    }
    log::debug!("Unmatched obligations {:#?}", peel_to_pred(&assoc_obls));

    let table = serialize_to_value(infcx, &table)
      .expect("couldn't serialize method lookup table");
    let unmarked = assoc_obls
      .into_iter()
      .map(|f| f.forget())
      .collect::<Vec<_>>();

    // ------------------------------------

    let source_map = self.tcx.sess.source_map();
    let range = CharRange::from_span(data.call_span, source_map)
      .expect("couldn't find range for span");

    Some(AmbiguityError {
      range,
      lookup: MethodLookup { table, unmarked },
    })
  }

  fn is_deref_of(
    &self,
    step: &CandidateStep<'tcx>,
    data: &FullObligationData<'tcx>,
  ) -> bool {
    let deref_def_id = self
      .tcx
      .lang_items()
      .deref_trait()
      .unwrap_or_else(|| self.tcx.require_lang_item(LangItem::Deref, None));

    if !self
      .tcx
      .is_trait_pred_rhs(&data.obligation.predicate, deref_def_id)
    {
      return false;
    }

    let trait_pred = data.obligation.predicate.expect_trait_predicate();

    // FIXME: this is wrong and won't work, the LHS is a canonicalized
    // query result and the RHS is a TY.
    step.step_ty() == trait_pred.self_ty()
  }

  fn is_trait_alias_for(
    &self,
    step: &CandidateStep<'tcx>,
    _deref_queries: &Option<Provenance<ObligationHash>>,
    obldata: &FullObligationData<'tcx>,
  ) -> bool {
    let PredicateKind::AliasRelate(t0, t1, _dir) =
      obldata.obligation.predicate.kind().skip_binder()
    else {
      return false;
    };

    t0.ty().map(|t| step.step_ty() == t).unwrap_or(false)
      || t1.ty().map(|t| step.step_ty() == t).unwrap_or(false)
  }

  fn is_trait_predicate_for(
    &self,
    step: &CandidateStep<'tcx>,
    obldata: &FullObligationData<'tcx>,
  ) -> bool {
    let Some(tp) = obldata.obligation.predicate.as_trait_predicate() else {
      return false;
    };

    step.step_ty() == tp.self_ty()
  }
}

trait PredicateExt<'tcx> {
  fn as_trait_predicate(&self) -> Option<TraitPredicate<'tcx>>;

  fn expect_trait_predicate(&self) -> TraitPredicate<'tcx>;
}

impl<'tcx> PredicateExt<'tcx> for Predicate<'tcx> {
  fn as_trait_predicate(&self) -> Option<TraitPredicate<'tcx>> {
    let clause = self.as_clause()?;
    let trait_ = clause.as_trait_clause()?;
    Some(trait_.skip_binder())
  }

  fn expect_trait_predicate(&self) -> TraitPredicate<'tcx> {
    self.as_trait_predicate().expect("expected trait predicate")
  }
}

trait CandidateStepExt<'tcx> {
  fn step_ty(&self) -> Ty<'tcx>;
}

impl<'tcx> CandidateStepExt<'tcx> for CandidateStep<'tcx> {
  fn step_ty(&self) -> Ty<'tcx> {
    self.self_ty.value.value
  }
}
