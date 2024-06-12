use rustc_infer::infer::InferCtxt;
use rustc_middle::{
  traits::solve::Goal,
  ty::{
    self, fold::BottomUpFolder, Predicate, Ty, TyCtxt, TypeFoldable,
    TypeVisitableExt, Upcast,
  },
};

use crate::{analysis::EvaluationResult, rustc::ImplCandidate};

#[derive(Debug)]
pub struct ImplementorInfo<'tcx> {
  implementor: ty::TraitRef<'tcx>,
  terrs: Vec<ty::error::TypeError<'tcx>>,
}

pub trait InferCtxtEvalExt<'tcx> {
  fn eval_goal(
    &self,
    obligation: Goal<'tcx, Predicate<'tcx>>,
  ) -> EvaluationResult;

  fn eval_with_self(
    &self,
    self_ty: Ty<'tcx>,
    trait_ref: ty::PolyTraitRef<'tcx>,
    param_env: ty::ParamEnv<'tcx>,
  ) -> EvaluationResult;

  fn find_implementing_type(
    &self,
    single: &ImplCandidate<'tcx>,
    trait_ref: ty::PolyTraitRef<'tcx>,
    param_env: ty::ParamEnv<'tcx>,
  ) -> Option<ImplementorInfo<'tcx>>;
}

impl<'tcx> InferCtxtEvalExt<'tcx> for InferCtxt<'tcx> {
  fn eval_goal(
    &self,
    obligation: Goal<'tcx, Predicate<'tcx>>,
  ) -> EvaluationResult {
    use rustc_trait_selection::{
      solve::{GenerateProofTree, InferCtxtEvalExt},
      traits::query::NoSolution,
    };
    self.probe(|_| {
      match self
        .evaluate_root_goal(obligation.clone().into(), GenerateProofTree::Never)
        .0
      {
        Ok((_, c)) => Ok(c),
        _ => Err(NoSolution),
      }
    })
  }

  fn eval_with_self(
    &self,
    self_ty: Ty<'tcx>,
    trait_ref: ty::PolyTraitRef<'tcx>,
    param_env: ty::ParamEnv<'tcx>,
  ) -> EvaluationResult {
    let tp = trait_ref.map_bound(|tp| tp.with_self_ty(self.tcx, self_ty));
    let goal = Goal {
      predicate: tp.upcast(self.tcx),
      param_env,
    };
    self.eval_goal(goal)
  }

  fn find_implementing_type(
    &self,
    single: &ImplCandidate<'tcx>,
    trait_ref: ty::PolyTraitRef<'tcx>,
    param_env: ty::ParamEnv<'tcx>,
  ) -> Option<ImplementorInfo<'tcx>> {
    use rustc_span::DUMMY_SP;
    use rustc_trait_selection::traits::{
      Obligation, ObligationCause, ObligationCtxt,
    };
    self.probe(|_| {
      let ocx = ObligationCtxt::new(self);
      let fresh_ty_var = self.next_ty_var(rustc_span::DUMMY_SP);
      let fresh_trait_ref = trait_ref
        .rebind(trait_ref.skip_binder().with_self_ty(self.tcx, fresh_ty_var));

      self.enter_forall(fresh_trait_ref, |obligation_trait_ref| {
        let impl_args = self.fresh_args_for_item(DUMMY_SP, single.impl_def_id);
        let impl_trait_ref = ocx.normalize(
          &ObligationCause::dummy(),
          param_env,
          ty::EarlyBinder::bind(single.trait_ref)
            .instantiate(self.tcx, impl_args),
        );

        ocx.register_obligations(
          self
            .tcx
            .predicates_of(single.impl_def_id)
            .instantiate(self.tcx, impl_args)
            .into_iter()
            .map(|(clause, _)| {
              Obligation::new(
                self.tcx,
                ObligationCause::dummy(),
                param_env,
                clause,
              )
            }),
        );

        if !ocx.select_where_possible().is_empty() {
          return None;
        }

        let mut terrs = vec![];
        for (obligation_arg, impl_arg) in
          std::iter::zip(obligation_trait_ref.args, impl_trait_ref.args)
        {
          if (obligation_arg, impl_arg).references_error() {
            return None;
          }
          if let Err(terr) = ocx.eq(
            &ObligationCause::dummy(),
            param_env,
            impl_arg,
            obligation_arg,
          ) {
            terrs.push(terr);
          }
          if !ocx.select_where_possible().is_empty() {
            return None;
          }
        }

        let cand = self.resolve_vars_if_possible(impl_trait_ref).fold_with(
          &mut BottomUpFolder {
            tcx: self.tcx,
            ty_op: |ty| ty,
            lt_op: |lt| lt,
            ct_op: |ct| ct.normalize(self.tcx, ty::ParamEnv::empty()),
          },
        );

        if cand.references_error() {
          return None;
        }

        Some(ImplementorInfo {
          implementor: cand,
          terrs,
        })
      })
    })
  }
}

pub fn is_local(ty: Ty) -> bool {
  match ty.kind() {
    ty::TyKind::Ref(_, ty, _) | ty::TyKind::RawPtr(ty, ..) => is_local(*ty),

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

pub fn function_arity<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<usize> {
  let from_def_id = |did| {
    Some(
      tcx
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
    ty::TyKind::Ref(_, ty, _) => function_arity(tcx, *ty),
    ty::TyKind::RawPtr(ty, _) => function_arity(tcx, *ty),
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

pub fn fn_trait_arity<'tcx>(
  _tcx: TyCtxt<'tcx>,
  t: ty::TraitPredicate<'tcx>,
) -> Option<usize> {
  let fn_arg_type = t.trait_ref.args.type_at(1);
  if let ty::TyKind::Tuple(tys) = fn_arg_type.kind() {
    Some(tys.len())
  } else {
    None
  }
}
