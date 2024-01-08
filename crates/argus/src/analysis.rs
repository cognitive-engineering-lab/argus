//! ProofTree analysis.
use anyhow::{anyhow, Result};
use fluid_let::fluid_let;
use itertools::Itertools;
use rustc_data_structures::{fx::FxIndexSet, stable_hasher::Hash64};
use rustc_hir::{def_id::LocalDefId, BodyId};
use rustc_hir_analysis::astconv::AstConv;
use rustc_hir_typeck::{inspect_typeck, FnCtxt};
use rustc_infer::{
  infer::error_reporting::TypeErrCtxt,
  traits::{util::elaborate, FulfilledObligation},
};
use rustc_middle::ty::{self, ToPolyTraitRef, TyCtxt};
use rustc_trait_selection::traits::{solve::Goal, FulfillmentError};
use rustc_utils::source_map::range::CharRange;

use crate::{
  rustc::FnCtxtExt as RustcFnCtxtExt,
  proof_tree::{
    ext::*, serialize::serialize_proof_tree, Obligation, ObligationKind,
    SerializedTree,
  },
  serialize::serialize_to_value,
  ObligationsInBody, TraitError, Target,
};

fluid_let!(pub static OBLIGATION_TARGET: Target);

// Data returned from endpoints

pub fn obligations<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
) -> Result<serde_json::Value> {
  let hir = tcx.hir();
  let local_def_id = hir.body_owner_def_id(body_id);

  log::info!("Getting obligations in body {}", {
    let owner = hir.body_owner(body_id);
    hir
      .opt_name(owner)
      .map(|s| s.to_string())
      .unwrap_or("<anon body>".to_string())
  });

  let mut result = Err(anyhow!("Hir Typeck never called inspect fn."));

  inspect_typeck(tcx, local_def_id, |fncx| {
    let Some(infcx) = fncx.infcx() else {
      return;
    };

    let obligations = fncx.get_obligations(local_def_id);
    let body = hir.body(body_id);
    let source_map = tcx.sess.source_map();
    let body_range =
      CharRange::from_span(body.value.span, source_map)
        .expect("Couldn't get body range");

    let trait_errors = infcx.reported_trait_errors.borrow().iter().flat_map(|(span, predicates)| {
      let range = CharRange::from_span(*span, source_map)
        .expect("Couldn't get trait bound range");
      predicates.iter().map(move |predicate| TraitError {
        range,
        predicate: predicate.clone(),
      })
    }).collect::<Vec<_>>();

    let obligation_in_body = ObligationsInBody {
      name: hir.opt_name(body_id.hir_id),
      range: body_range,
      ambiguity_errors: vec![],
      trait_errors,
      obligations,
    };

    result = serialize_to_value(&obligation_in_body, infcx)
      .map_err(|e| anyhow!(e));
  });

  result
}

// NOTE: tree is only invoked for *a single* tree, it must be found
// within the `body_id` and the appropriate `OBLIGATION_TARGET` (i.e., stable hash).
pub fn tree<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
) -> Result<serde_json::Value> {
  OBLIGATION_TARGET.get(|target| {
    let target = target.unwrap();

    let hir = tcx.hir();
    let local_def_id = hir.body_owner_def_id(body_id);
    let mut result =
      Err(anyhow!("Couldn't find proof tree with hash {:?}", target));

    inspect_typeck(tcx, local_def_id, |fncx| {
      let Some(infcx) = fncx.infcx() else {
        return;
      };

      let errors = fncx.get_fulfillment_errors(local_def_id);
      let found_tree = errors.iter().find_map(|(hash, error)| {
        if hash.as_u64() != target.hash {
          return None;
        }

        let goal = Goal {
          predicate: error.root_obligation.predicate,
          param_env: error.root_obligation.param_env,
        };
        let serial_tree = serialize_tree(goal, fncx)?;
        serialize_to_value(&serial_tree, infcx).ok()
      });

      result = found_tree
        .ok_or(anyhow!("Couldn't find proof tree with hash {:?}", target));
    });

    result
  })
}

fn serialize_tree<'tcx>(
  goal: Goal<'tcx, ty::Predicate<'tcx>>,
  fcx: &FnCtxt<'_, 'tcx>,
) -> Option<SerializedTree<'tcx>> {
  let def_id = fcx.item_def_id();
  let infcx = fcx.infcx().expect("`FnCtxt` missing a `InferCtxt`.");

  serialize_proof_tree(goal, infcx, def_id)
}

fn retain_fixpoint<T, F: FnMut(&T) -> bool>(v: &mut Vec<T>, mut pred: F) {
  // NOTE: the original intent was to keep a single element, but that doesn't seem
  // to be ideal. Perhaps it's best to remove all elements and then allow users to
  // toggle these "hidden elements" should they choose to.
  let keep_n_elems = 0;
  let mut did_change = true;
  let start_size = v.len();
  let mut removed_es = 0usize;
  // While things have changed, keep iterating, except
  // when we have a single element left.
  while did_change && start_size - removed_es > keep_n_elems {
    did_change = false;
    v.retain(|e| {
      let r = pred(e);
      did_change |= !r;
      if !r && start_size - removed_es > keep_n_elems {
        removed_es += 1;
        r
      } else {
        true
      }
    });
  }
}

// --------------------------------

pub(crate) type FulfillmentData<'tcx> = (Hash64, FulfillmentError<'tcx>);

trait FnCtxtExt<'tcx> {
  fn get_obligations(&self, ldef_id: LocalDefId) -> Vec<Obligation<'tcx>>;

  fn get_fulfillment_errors(
    &self,
    ldef_id: LocalDefId,
  ) -> Vec<FulfillmentData<'tcx>>;

  fn convert_fulfillment_errors(
    &self,
    errors: Vec<FulfillmentData<'tcx>>,
  ) -> Vec<Obligation<'tcx>>;
}

impl<'tcx> FnCtxtExt<'tcx> for FnCtxt<'_, 'tcx> {
  fn get_obligations(&self, ldef_id: LocalDefId) -> Vec<Obligation<'tcx>> {
    let mut errors = self.get_fulfillment_errors(ldef_id);
    self.adjust_fulfillment_errors_for_expr_obligation(&mut errors);
    self.convert_fulfillment_errors(errors)
  }

  fn get_fulfillment_errors(
    &self,
    ldef_id: LocalDefId,
  ) -> Vec<FulfillmentData<'tcx>> {
    let infcx = self.infcx().unwrap();

    let return_with_hashes = |v: Vec<FulfillmentError<'tcx>>| {
      self.tcx().with_stable_hashing_context(|mut hcx| {
        v.into_iter()
         .map(|e| (e.stable_hash(infcx, &mut hcx), e))
         .unique_by(|(h, _)| *h)
         .collect::<Vec<_>>()
      })
    };

    let mut result = Vec::new();

    let def_id = ldef_id.to_def_id();

    if let Some(infcx) = self.infcx() {
      let fulfilled_obligations = infcx.fulfilled_obligations.borrow();
      let _tcx = &infcx.tcx;

      result.extend(fulfilled_obligations.iter().filter_map(|obl| match obl {
        FulfilledObligation::Failure(error) => Some(error.clone()),
        FulfilledObligation::Success(obl) => None,
      }));
    }

    let tcx = &self.tcx();
    // NOTE: this will remove everything that is not "necessary,"
    // below might be a better strategy. The best is ordering them by
    // relevance and then hiding unnecessary obligations unless the
    // user wants to see them.
    retain_fixpoint(&mut result, |error| {
      error.obligation.predicate.is_necessary(tcx)
    });

    // Iteratively filter out elements unless there's only one thing
    // left; we don't want to remove the last remaining query.
    // Queries in order of *least* importance:
    // 1. (): TRAIT
    // 2. TY: Sized
    // 3. _: TRAIT
    // retain_fixpoint(&mut result, |error| {
    //   !error.obligation.predicate.is_unit_impl_trait(tcx)
    // });

    // retain_fixpoint(&mut result, |error| {
    //   !error.obligation.predicate.is_ty_impl_sized(tcx)
    // });

    // retain_fixpoint(&mut result, |error| {
    //   !error.obligation.predicate.is_ty_unknown(tcx)
    // });

    return_with_hashes(result)
  }


  fn convert_fulfillment_errors(
    &self,
    errors: Vec<FulfillmentData<'tcx>>,
  ) -> Vec<Obligation<'tcx>> {
    if errors.is_empty() {
      return Vec::new();
    }
    let source_map = self.tcx().sess.source_map();
    let _infcx = self.infcx().unwrap();

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

    errors
      .into_iter()
      .map(|(hash, error)| {
        let predicate = error.root_obligation.predicate;
        let range =
          CharRange::from_span(error.obligation.cause.span, source_map)
          .unwrap();
        Obligation {
          data: predicate,
          hash: hash.as_u64().to_string(),
          range,
          kind: ObligationKind::Failure,
        }
      })
      .collect::<Vec<_>>()
  }
}
