//! ProofTree analysis.

use rustc_data_structures::{fx::FxIndexSet, stable_hasher::Hash64};
use rustc_hir::BodyId;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir_analysis::astconv::AstConv;
use rustc_hir_typeck::{inspect_typeck, FnCtxt};
use rustc_infer::infer::error_reporting::TypeErrCtxt;
use rustc_infer::traits::FulfilledObligation;
use rustc_infer::traits::util::elaborate;
use rustc_middle::ty::{self, TyCtxt, ToPolyTraitRef};
use rustc_span::symbol::Symbol;
use rustc_trait_selection::traits::FulfillmentError;
use rustc_trait_selection::traits::solve::Goal;

use anyhow::{Result, anyhow};
use fluid_let::fluid_let;
use rustc_utils::source_map::range::CharRange;
use serde::Serialize;
use ts_rs::TS;
use itertools::Itertools;

use crate::Target;
use crate::proof_tree::{SerializedTree, Obligation, ObligationKind};
use crate::proof_tree::ext::*;
use crate::proof_tree::serialize::serialize_proof_tree;
use crate::ty::SymbolDef;

fluid_let!(pub static OBLIGATION_TARGET: Target);

// Data returned from endpoints

#[derive(Serialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
pub struct ObligationsInBody {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(serialize_with = "serialize_option")]
  #[cfg_attr(feature = "ts-rs", ts(type = "SymbolDef?"))]
  name: Option<Symbol>,

  range: CharRange,

  // HACK it's easiest to already convert Obligations
  // to a JSON Value to avoid having lifetimes in the
  // plugin endpoint.
  #[cfg_attr(feature = "ts-rs", ts(type = "Obligation[]"))]
  obligations: serde_json::Value,
}

pub fn obligations<'tcx>(tcx: TyCtxt<'tcx>, body_id: BodyId) -> Result<ObligationsInBody>
{
  let hir = tcx.hir();
  let local_def_id = hir.body_owner_def_id(body_id);

  log::info!("Getting obligations in body {}", {
    let owner = hir.body_owner(body_id);
    hir.opt_name(owner).map(|s| s.to_string()).unwrap_or("<anon>".to_string())
  });

  let mut result = Err(anyhow!("Hir Typeck never called inspect fn."));

  inspect_typeck(tcx, local_def_id, |fncx| {
    let Some(infcx) = fncx.infcx() else {
      return;
    };

    let obligations = fncx.get_obligations(local_def_id);
    let json = crate::ty::serialize_to_value(&obligations, infcx)
      .expect("Could not serialize Obligations");
    let body = hir.body(body_id);
    let body_range = CharRange::from_span(body.value.span, tcx.sess.source_map())
      .expect("Couldn't get body range");
    result = Ok(ObligationsInBody {
      name: hir.opt_name(body_id.hir_id),
      range: body_range,
      obligations: json,
    });
  });

  result
}

// NOTE: tree is only invoked for *a single* tree, it must be found
// within the `body_id` and the appropriate `OBLIGATION_TARGET` (i.e., stable hash).
pub fn tree<'tcx>(tcx: TyCtxt<'tcx>, body_id: BodyId) -> Result<serde_json::Value>
{
  OBLIGATION_TARGET.get(|target| {
    let target = target.unwrap();

    let hir = tcx.hir();
    let local_def_id = hir.body_owner_def_id(body_id);
    let mut result = Err(anyhow!("Couldn't find proof tree with hash {:?}", target));

    inspect_typeck(tcx, local_def_id, |fncx| {
      let Some(infcx) = fncx.infcx() else {
        return;
      };

      let errors = fncx.get_fulfillment_errors(local_def_id);
      let found_tree = errors.iter().find_map(|(hash, error)| {
        if hash.as_u64() != target.hash {
          return None;
        }

        let goal =
          Goal { predicate: error.root_obligation.predicate, param_env: error.root_obligation.param_env };
        let serial_tree = serialize_tree(goal, fncx)?;
        crate::ty::serialize_to_value(&serial_tree, infcx).ok()
      });

      result = found_tree.ok_or(anyhow!("Couldn't find proof tree with hash {:?}", target));
    });

    result
  })
}

fn serialize_tree<'tcx>(goal: Goal<'tcx, ty::Predicate<'tcx>>, fcx: &FnCtxt<'_, 'tcx>) -> Option<SerializedTree<'tcx>> {
  let def_id = fcx.item_def_id();
  let infcx = fcx.infcx().expect("`FnCtxt` missing a `InferCtxt`.");

  serialize_proof_tree(goal, infcx, def_id)
}

fn retain_fixpoint<T, F: FnMut(&T) -> bool>(v: &mut Vec<T>, mut pred: F) {
  let mut did_change = true;
  let start_size = v.len();
  let mut removed_es = 0usize;
  // While things have changed, keep iterating, except
  // when we have a single element left.
  while did_change && start_size - removed_es > 1 {
    did_change = false;
    v.retain(|e| {
      let r = pred(e);
      did_change |= !r;
      if !r && start_size - removed_es > 1 {
        removed_es += 1;
        r
      } else {
        true
      }
    });
  }
}

// --------------------------------

type FulfillmentData<'tcx> = (Hash64, FulfillmentError<'tcx>);

trait FnCtxtExt<'tcx> {
  fn get_fulfillment_errors(&self, ldef_id: LocalDefId) -> Vec<FulfillmentData<'tcx>>;
  fn get_obligations(&self, ldef_id: LocalDefId) -> Vec<Obligation<'tcx>>;
  fn adjust_fulfillment_errors_for_expr_obligation(&self, errors: &mut Vec<FulfillmentData<'tcx>>);
  fn report_fulfillment_errors(&self, errors: Vec<FulfillmentData<'tcx>>) -> Vec<Obligation<'tcx>>;
}

trait InferPrivateExt<'tcx> {
  fn error_implies(&self, cond: ty::Predicate<'tcx>, error: ty::Predicate<'tcx>) -> bool;
}

// Taken from rustc_trait_selection/src/traits/error_reporting/type_err_ctxt_ext.rs
impl<'tcx> InferPrivateExt<'tcx> for TypeErrCtxt<'_, 'tcx> {
  fn error_implies(&self, cond: ty::Predicate<'tcx>, error: ty::Predicate<'tcx>) -> bool {
    use log::debug;

    if cond == error {
      return true;
    }

    // FIXME: It should be possible to deal with `ForAll` in a cleaner way.
    let bound_error = error.kind();
    let (cond, error) = match (cond.kind().skip_binder(), bound_error.skip_binder()) {
      (
        ty::PredicateKind::Clause(ty::ClauseKind::Trait(..)),
        ty::PredicateKind::Clause(ty::ClauseKind::Trait(error)),
      ) => (cond, bound_error.rebind(error)),
      _ => {
        // FIXME: make this work in other cases too.
        return false;
      }
    };

    for pred in elaborate(self.tcx, std::iter::once(cond)) {
      let bound_predicate = pred.kind();
      if let ty::PredicateKind::Clause(ty::ClauseKind::Trait(implication)) =
        bound_predicate.skip_binder()
      {
        let error = error.to_poly_trait_ref();
        let implication = bound_predicate.rebind(implication.trait_ref);
        // FIXME: I'm just not taking associated types at all here.
        // Eventually I'll need to implement param-env-aware
        // `Γ₁ ⊦ φ₁ => Γ₂ ⊦ φ₂` logic.
        let param_env = ty::ParamEnv::empty();
        if self.can_sub(param_env, error, implication) {
          debug!("error_implies: {:?} -> {:?} -> {:?}", cond, error, implication);
          return true;
        }
      }
    }

    false
  }
}

impl<'tcx> FnCtxtExt<'tcx> for FnCtxt<'_, 'tcx> {
  fn get_obligations(&self, ldef_id: LocalDefId) -> Vec<Obligation<'tcx>> {
      let mut errors = self.get_fulfillment_errors(ldef_id);
      self.adjust_fulfillment_errors_for_expr_obligation(&mut errors);
      self.report_fulfillment_errors(errors)
  }

  fn get_fulfillment_errors(&self, ldef_id: LocalDefId) -> Vec<FulfillmentData<'tcx>> {
    let infcx = self.infcx().unwrap();

    let return_with_hashes = |v: Vec<FulfillmentError<'tcx>>| {
      self.tcx().with_stable_hashing_context(|mut hcx| {
        v
          .into_iter()
          .map(|e| (e.stable_hash(infcx, &mut hcx), e))
          .unique_by(|(h, _)| *h)
          .collect::<Vec<_>>()
      })
    };

    let mut result = Vec::new();

    let def_id = ldef_id.to_def_id();

    if let Some(infcx) = self.infcx() {
      let fulfilled_obligations = infcx.fulfilled_obligations.borrow();
      let tcx = &infcx.tcx;

      result.extend(
        fulfilled_obligations.iter().filter_map(|obl| {

          match obl {
            FulfilledObligation::Failure(error) => {
              log::debug!("[CAND] error {:?}", error.obligation.predicate.pretty(infcx, def_id));
              Some(error.clone())
            },
            FulfilledObligation::Success(obl) => {
              log::debug!("[CAND] success {:?}", obl.predicate.pretty(infcx, def_id));
              None
            }
          }
        }));
    }

    // Iteratively filter out elements unless there's only one thing
    // left; we don't want to remove the last remaining query.
    // Queries in order of *least* importance:
    // 1. (): TRAIT
    // 2. TY: Sized
    // 3. _: TRAIT
    let tcx = &self.tcx();
    retain_fixpoint(&mut result, |error| {
      !error.obligation.predicate.is_unit_impl_trait(tcx)
    });

    retain_fixpoint(&mut result, |error| {
      !error.obligation.predicate.is_ty_impl_sized(tcx)
    });

    retain_fixpoint(&mut result, |error| {
      !error.obligation.predicate.is_ty_unknown(tcx)
    });

    return_with_hashes(result)
  }

  // Implementation taken from rustc_hir_typeck/fn_ctxt/checks.rs :: adjust_fulfillment_errors_for_expr_obligation
  fn adjust_fulfillment_errors_for_expr_obligation(&self, errors: &mut Vec<FulfillmentData<'tcx>>) {

    let mut remap_cause = FxIndexSet::default();
    let mut not_adjusted = vec![];

    for (_, error) in errors {
      let before_span = error.obligation.cause.span;
      if self.adjust_fulfillment_error_for_expr_obligation(error)
        || before_span != error.obligation.cause.span
      {
        remap_cause.insert((
          before_span,
          error.obligation.predicate,
          error.obligation.cause.clone(),
        ));
      } else {
        not_adjusted.push(error);
      }
    }

    for error in not_adjusted {
      for (span, predicate, cause) in &remap_cause {
        if *predicate == error.obligation.predicate
          && span.contains(error.obligation.cause.span)
        {
          error.obligation.cause = cause.clone();
          continue;
        }
      }
    }
  }

  fn report_fulfillment_errors(&self, mut errors: Vec<FulfillmentData<'tcx>>) -> Vec<Obligation<'tcx>> {
    if errors.is_empty() {
      return Vec::new();
    }
    let source_map = self.tcx().sess.source_map();
    let infcx = self.infcx().unwrap();

    // let this = self.err_ctxt();

    // let reported = this
    //   .reported_trait_errors
    //   .borrow()
    //   .iter()
    //   .flat_map(|(_, ps)| {
    //     ps.iter().copied()
    //   })
    //   .collect::<Vec<_>>();

    // // FIXME
    // let _split_idx = itertools::partition(&mut errors, |error| {
    //   reported.iter().any(|p| *p == error.obligation.predicate)
    // });

    // let reported_errors = this.reported_trait_errors.borrow();

    // log::debug!("Reported_errors {_split_idx} {reported_errors:#?}");

    errors.into_iter().map(|(hash, error)| {
      let predicate = error.root_obligation.predicate;
      let range = CharRange::from_span(error.obligation.cause.span, source_map).unwrap();
      Obligation {
        data: predicate,
        hash: hash.as_u64().to_string(),
        range,
        kind: ObligationKind::Failure
      }
    }).collect::<Vec<_>>()
  }
}

// Serialize an Option<Symbol> using SymbolDef but the value must be a Some(..)
fn serialize_option<S: serde::Serializer>(value: &Option<Symbol>, s: S) -> Result<S::Ok, S::Error> {
  let Some(symb) = value else {
    unreachable!();
  };

  SymbolDef::serialize(symb, s)
}
