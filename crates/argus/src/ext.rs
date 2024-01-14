use itertools::Itertools;
use rustc_hir::def_id::LocalDefId;
use rustc_hir_analysis::astconv::AstConv;
use rustc_hir_typeck::FnCtxt;
use rustc_infer::{infer::InferCtxt, traits::FulfilledObligation};
use rustc_trait_selection::traits::FulfillmentError;
use rustc_utils::source_map::range::CharRange;

use crate::{
  analysis::FulfillmentData,
  proof_tree::{ext::*, Obligation, ObligationKind},
  rustc::FnCtxtExt as RustcFnCtxtExt,
  types::{AmbiguityError, TraitError},
};

pub trait CharRangeExt: Copy + Sized {
  /// Returns true if this range touches the `other`.
  fn overlaps(self, other: Self) -> bool;
}

pub trait FnCtxtExt<'tcx> {
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

pub trait InferCtxtExt<'tcx> {
  fn build_trait_errors(
    &self,
    obligations: &[Obligation<'tcx>],
  ) -> Vec<TraitError<'tcx>>;

  // TODO: This might need to go on the FnCtxt
  fn build_ambiguity_errors(
    &self,
    obligations: &[Obligation<'tcx>],
  ) -> Vec<AmbiguityError<'tcx>>;
}

impl CharRangeExt for CharRange {
  fn overlaps(self, other: Self) -> bool {
    self.start < other.end && other.start < self.end
  }
}

impl<'tcx> InferCtxtExt<'tcx> for InferCtxt<'tcx> {
  fn build_trait_errors(
    &self,
    obligations: &[Obligation<'tcx>],
  ) -> Vec<TraitError<'tcx>> {
    let tcx = &self.tcx;
    let source_map = tcx.sess.source_map();
    self
      .reported_trait_errors
      .borrow()
      .iter()
      .flat_map(|(span, predicates)| {
        let range = CharRange::from_span(*span, source_map)
          .expect("Couldn't get trait bound range");

        predicates.iter().map(move |predicate| {
          // TODO: these simple comparisons are not going to cut it ...
          // We can always take a similar approach to the ambiguity errors and
          // just recompute the errors that rustc does.
          //
          // Another idea would be to use the "X implies Y" mechanism from the
          // diagnostic system. This will collapse all implied errors into the one reported.
          let candidates = obligations
            .iter()
            .filter_map(|obl| {
              if !range.overlaps(obl.range) || obl.predicate != *predicate {
                return None;
              }
              Some(obl.hash)
            })
            .collect::<Vec<_>>();

          TraitError {
            range,
            predicate: predicate.clone(),
            candidates,
          }
        })
      })
      .collect::<Vec<_>>()
  }

  fn build_ambiguity_errors(
    &self,
    _obligations: &[Obligation<'tcx>],
  ) -> Vec<AmbiguityError<'tcx>> {
    todo!()
  }
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
    use rustc_hir_typeck::Inherited;
    let infcx = self.infcx().unwrap();

    let return_with_hashes = |v: Vec<FulfillmentError<'tcx>>| {
      self.tcx().with_stable_hashing_context(|mut hcx| {
        v.into_iter()
          .map(|e| (e.stable_hash(infcx, &mut hcx), e))
          .unique_by(|(h, _)| *h)
          .collect::<Vec<_>>()
      })
    };

    // NON-Updated code (with-probes)
    let mut result = Vec::new();
    let _def_id = ldef_id.to_def_id();
    if let Some(infcx) = self.infcx() {
      let fulfilled_obligations = infcx.fulfilled_obligations.borrow();
      result.extend(fulfilled_obligations.iter().filter_map(|obl| match obl {
        FulfilledObligation::Failure(error) => Some(error.clone()),
        FulfilledObligation::Success(_obl) => None,
      }));
    }

    // Updated code (sans-probes)
    // let inh: &Inherited<'tcx> = self;
    // let engine = inh.get_engine();
    // let fulfilled_obligations = engine.get_tracked_obligations().unwrap();
    // let mut result = fulfilled_obligations
    //   .iter()
    //   .filter_map(|obl| match obl {
    //     FulfilledObligation::Failure(error) => Some(error.clone()),
    //     FulfilledObligation::Success(_obl) => None,
    //   })
    //   .collect::<Vec<_>>();

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
          predicate,
          hash: hash.into(),
          range,
          kind: ObligationKind::Failure,
        }
      })
      .collect::<Vec<_>>()
  }
}

pub fn retain_fixpoint<T, F: FnMut(&T) -> bool>(v: &mut Vec<T>, mut pred: F) {
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
