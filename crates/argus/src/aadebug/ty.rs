// use rustc_infer::infer::InferCtxt;
// use rustc_middle::{
//   traits::solve::Goal,
//   ty::{
//     self, fold::BottomUpFolder, Predicate, Ty, TyCtxt, TypeFoldable,
//     TypeVisitableExt, Upcast,
//   },
// };

// use crate::{analysis::EvaluationResult, rustc::ImplCandidate};

// #[derive(Debug)]
// pub struct ImplementorInfo<'tcx> {
//   implementor: ty::TraitRef<'tcx>,
//   terrs: Vec<ty::error::TypeError<'tcx>>,
// }

// pub trait InferCtxtEvalExt<'tcx> {
//   fn eval_goal(
//     &self,
//     obligation: Goal<'tcx, Predicate<'tcx>>,
//   ) -> EvaluationResult;

//   fn eval_with_self(
//     &self,
//     self_ty: Ty<'tcx>,
//     trait_ref: ty::PolyTraitRef<'tcx>,
//     param_env: ty::ParamEnv<'tcx>,
//   ) -> EvaluationResult;

//   fn find_implementing_type(
//     &self,
//     single: &ImplCandidate<'tcx>,
//     trait_ref: ty::PolyTraitRef<'tcx>,
//     param_env: ty::ParamEnv<'tcx>,
//   ) -> Option<ImplementorInfo<'tcx>>;
// }

// impl<'tcx> InferCtxtEvalExt<'tcx> for InferCtxt<'tcx> {
//   fn eval_goal(
//     &self,
//     obligation: Goal<'tcx, Predicate<'tcx>>,
//   ) -> EvaluationResult {
//     use rustc_trait_selection::{
//       solve::{GenerateProofTree, InferCtxtEvalExt},
//       traits::query::NoSolution,
//     };
//     self.probe(|_| {
//       match self
//         .evaluate_root_goal(obligation.clone().into(), GenerateProofTree::Never)
//         .0
//       {
//         Ok((_, c)) => Ok(c),
//         _ => Err(NoSolution),
//       }
//     })
//   }

//   fn eval_with_self(
//     &self,
//     self_ty: Ty<'tcx>,
//     trait_ref: ty::PolyTraitRef<'tcx>,
//     param_env: ty::ParamEnv<'tcx>,
//   ) -> EvaluationResult {
//     let tp = trait_ref.map_bound(|tp| tp.with_self_ty(self.tcx, self_ty));
//     let goal = Goal {
//       predicate: tp.upcast(self.tcx),
//       param_env,
//     };
//     self.eval_goal(goal)
//   }

//   fn find_implementing_type(
//     &self,
//     single: &ImplCandidate<'tcx>,
//     trait_ref: ty::PolyTraitRef<'tcx>,
//     param_env: ty::ParamEnv<'tcx>,
//   ) -> Option<ImplementorInfo<'tcx>> {
//     use rustc_span::DUMMY_SP;
//     use rustc_trait_selection::traits::{
//       Obligation, ObligationCause, ObligationCtxt,
//     };
//     self.probe(|_| {
//       let ocx = ObligationCtxt::new(self);
//       let fresh_ty_var = self.next_ty_var(rustc_span::DUMMY_SP);
//       let fresh_trait_ref = trait_ref
//         .rebind(trait_ref.skip_binder().with_self_ty(self.tcx, fresh_ty_var));

//       self.enter_forall(fresh_trait_ref, |obligation_trait_ref| {
//         let impl_args = self.fresh_args_for_item(DUMMY_SP, single.impl_def_id);
//         let impl_trait_ref = ocx.normalize(
//           &ObligationCause::dummy(),
//           param_env,
//           ty::EarlyBinder::bind(single.trait_ref)
//             .instantiate(self.tcx, impl_args),
//         );

//         ocx.register_obligations(
//           self
//             .tcx
//             .predicates_of(single.impl_def_id)
//             .instantiate(self.tcx, impl_args)
//             .into_iter()
//             .map(|(clause, _)| {
//               Obligation::new(
//                 self.tcx,
//                 ObligationCause::dummy(),
//                 param_env,
//                 clause,
//               )
//             }),
//         );

//         if !ocx.select_where_possible().is_empty() {
//           return None;
//         }

//         let mut terrs = vec![];
//         for (obligation_arg, impl_arg) in
//           std::iter::zip(obligation_trait_ref.args, impl_trait_ref.args)
//         {
//           if (obligation_arg, impl_arg).references_error() {
//             return None;
//           }
//           if let Err(terr) = ocx.eq(
//             &ObligationCause::dummy(),
//             param_env,
//             impl_arg,
//             obligation_arg,
//           ) {
//             terrs.push(terr);
//           }
//           if !ocx.select_where_possible().is_empty() {
//             return None;
//           }
//         }

//         let cand = self.resolve_vars_if_possible(impl_trait_ref).fold_with(
//           &mut BottomUpFolder {
//             tcx: self.tcx,
//             ty_op: |ty| ty,
//             lt_op: |lt| lt,
//             ct_op: |ct| ct.normalize(self.tcx, ty::ParamEnv::empty()),
//           },
//         );

//         if cand.references_error() {
//           return None;
//         }

//         Some(ImplementorInfo {
//           implementor: cand,
//           terrs,
//         })
//       })
//     })
//   }
// }
