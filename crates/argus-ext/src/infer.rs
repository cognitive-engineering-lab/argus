use rustc_data_structures::stable_hasher::Hash64;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::ty::{self, Predicate, TypeFoldable};
use rustc_trait_selection::{
  solve::{GenerateProofTree, InferCtxtEvalExt},
  traits::query::NoSolution,
};

use crate::{ty::TyCtxtExt, EvaluationResult};

pub trait InferCtxtExt<'tcx> {
  fn sanitize_obligation(
    &self,
    typeck_results: &'tcx ty::TypeckResults<'tcx>,
    obligation: &PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> PredicateObligation<'tcx>;

  fn predicate_hash(&self, p: &Predicate<'tcx>) -> Hash64;

  fn evaluate_obligation(
    &self,
    obligation: &PredicateObligation<'tcx>,
  ) -> EvaluationResult;
}

impl<'tcx> InferCtxtExt<'tcx> for InferCtxt<'tcx> {
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

  fn evaluate_obligation(
    &self,
    obligation: &PredicateObligation<'tcx>,
  ) -> EvaluationResult {
    match self
      .evaluate_root_goal(obligation.clone().into(), GenerateProofTree::Never)
      .0
    {
      Ok((_, c)) => Ok(c),
      _ => Err(NoSolution),
    }
  }
}
