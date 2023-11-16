use rustc_hash::FxHashMap as HashMap;
use rustc_hir::{BodyId, FnSig};
use rustc_hir_analysis::astconv::AstConv;
use rustc_hir_typeck::{FnCtxt, Inherited};
use rustc_infer::infer::InferCtxt;
use rustc_middle::ty::{TyCtxt, Predicate};
use rustc_trait_selection::{infer::TyCtxtInferExt, traits::{ObligationCtxt, FulfillmentError, solve::{Goal, QueryInput}}, solve::inspect::{ProofTreeInferCtxtExt, ProofTreeVisitor, InspectGoal}};
use rustc_type_ir::Canonical;

pub type CanonicalGoal<'tcx> = Canonical<TyCtxt<'tcx>, QueryInput<'tcx, Predicate<'tcx>>>;

pub trait InspectGoalExt<'tcx> {
    fn canonical_goal(&self) -> CanonicalGoal<'tcx>;
}

// impl<'tcx> InspectGoalExt<'tcx> for InspectGoal<'_, 'tcx> {
//     fn canonical_goal(&self) -> &CanonicalGoal<'tcx> {
//         &self.evaluation.evaluation.goal
//     }
// }
