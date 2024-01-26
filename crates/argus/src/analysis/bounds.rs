use rustc_span::Span;
use rustc_utils::source_map::range::CharRange;

use crate::{
  analysis::{entry::ErrorAssemblyCtx, tls::RawTraitErrInfo},
  types::TraitError,
};

impl<'a, 'tcx: 'a> ErrorAssemblyCtx<'a, 'tcx> {
  // TODO: When is an obligaion related to a trait bound error.
  //
  // Let's say that that we have Bound error (S, P), where S is a Span and P is a Predicate.
  //
  // We have a list of bound errors (S_0, S_1, ..., P_0, P_1, ...)
  //
  // 1. Sort the list of bound errors by Span, smallest to largest. NOTE: really we want to sort
  //    the list by "innermost" to "outermost," but smallest to largest *should* suffice for now.
  //
  // 2. For each span S_i, we take (drain) the obligations that originated from an
  //    expression *fully contained* in S_i.
  //
  // XXX: each error should have an associated obligation, those obligations may be deemed
  // unnecessary later, but we can sort that out.
  pub fn assemble_bound_errors(
    &mut self,
    mut errs: RawTraitErrInfo,
  ) -> Vec<TraitError> {
    let _hir = self.tcx.hir();
    let source_map = self.tcx.sess.source_map();
    let sz = |s: Span| s.hi() - s.lo();

    errs.sort_by(|sa, _, sb, _| sz(*sa).cmp(&sz(*sb)));

    let mut terrs: Vec<TraitError> = vec![];
    for (span, (p, cans)) in errs.into_iter() {
      let range = CharRange::from_span(span, source_map)
        .expect("couldn't get trait error range");

      let candidates = self
        .obligations
        .extract_if(|poh| {
          cans.contains(&poh) || poh.contained_in(&self.tcx, span)
        })
        .map(|p| p.forget())
        .collect::<Vec<_>>();

      assert!(
        !candidates.is_empty(),
        "each bound error should have candidate obligations"
      );

      terrs.push(TraitError {
        range,
        candidates,
        predicate: p,
      });
    }

    terrs
  }
}
