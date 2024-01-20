use rustc_data_structures::stable_hasher::{Hash64, HashStable, StableHasher};
use rustc_hir::{def_id::LocalDefId, LangItem};
use rustc_hir_analysis::astconv::AstConv;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::ty::{
  self, Predicate, Ty, TyCtxt, TypeFoldable, TypeFolder, TypeSuperFoldable,
};
use rustc_query_system::ich::StableHashingContext;
use rustc_utils::source_map::range::CharRange;
use serde::Serialize;

use crate::{
  analysis::{EvaluationResult, FulfillmentData},
  serialize::{serialize_to_value, ty::PredicateDef},
  types::Obligation,
};

pub trait CharRangeExt: Copy + Sized {
  /// Returns true if this range touches the `other`.
  fn overlaps(self, other: Self) -> bool;
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

pub trait InferCtxtExt<'tcx> {
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

  fn is_necessary_predicate(&self, p: &Predicate<'tcx>) -> bool;
  fn is_unit_impl_trait(&self, p: &Predicate<'tcx>) -> bool;
  fn is_ty_impl_sized(&self, p: &Predicate<'tcx>) -> bool;
  fn is_ty_unknown(&self, p: &Predicate<'tcx>) -> bool;
  fn is_trait_predicate(&self, p: &Predicate<'tcx>) -> bool;

  fn body_id(&self) -> Option<LocalDefId>;

  fn predicate_hash(&self, p: &Predicate<'tcx>) -> Hash64;

  //   fn build_trait_errors(
  //     &self,
  //     obligations: &[Obligation<'tcx>],
  //   ) -> Vec<TraitError<'tcx>>;

  //   fn build_ambiguity_errors(
  //     &self,
  //     obligations: &[Obligation<'tcx>],
  //   ) -> Vec<AmbiguityError>;
}

// -----------------------------------------------
// Impls

impl CharRangeExt for CharRange {
  fn overlaps(self, other: Self) -> bool {
    self.start < other.end && other.start < self.end
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

impl<'tcx> InferCtxtExt<'tcx> for InferCtxt<'tcx> {
  fn is_unit_impl_trait(&self, p: &Predicate<'tcx>) -> bool {
    matches!(p.kind().skip_binder(),
    ty::PredicateKind::Clause(ty::ClauseKind::Trait(trait_predicate)) if {
        trait_predicate.self_ty().is_unit()
    })
  }

  fn is_ty_impl_sized(&self, p: &Predicate<'tcx>) -> bool {
    matches!(p.kind().skip_binder(),
    ty::PredicateKind::Clause(ty::ClauseKind::Trait(trait_predicate)) if {
        trait_predicate.def_id() == self.tcx.require_lang_item(LangItem::Sized, None)
    })
  }

  // TODO: I'm not 100% that this is the correct metric.
  fn is_ty_unknown(&self, p: &Predicate<'tcx>) -> bool {
    matches!(p.kind().skip_binder(),
    ty::PredicateKind::Clause(ty::ClauseKind::Trait(trait_predicate)) if {
        trait_predicate.self_ty().is_ty_var()
    })
  }

  fn is_trait_predicate(&self, p: &Predicate<'tcx>) -> bool {
    matches!(
      p.kind().skip_binder(),
      ty::PredicateKind::Clause(ty::ClauseKind::Trait(..))
    )
  }

  fn is_necessary_predicate(&self, p: &Predicate<'tcx>) -> bool {
    // NOTE: predicates of the form `_: TRAIT` and `(): TRAIT` are useless. The first doesn't have
    // any information about the type of the Self var, and I've never understood why the latter
    // occurs so frequently.
    self.is_trait_predicate(p)
      && !(self.is_unit_impl_trait(p)
        || self.is_ty_unknown(p)
        || self.is_ty_impl_sized(p))
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
    let source_map = self.tcx.sess.source_map();
    let range = CharRange::from_span(obl.cause.span, source_map)
      .expect("couldn't convert obligation span to range");
    let is_necessary = self.is_necessary_predicate(&obl.predicate);

    #[derive(Serialize)]
    struct Wrapper<'tcx>(#[serde(with = "PredicateDef")] Predicate<'tcx>);

    let predicate = if is_necessary {
      let w = Wrapper(obl.predicate.clone());
      serialize_to_value(self, &w).expect("could not serialize predicate")
    } else {
      serialize_to_value(self, &()).expect("could not serialize predicate")
    };

    Obligation {
      predicate,
      hash: fdata.hash.into(),
      range,
      kind: fdata.kind(),
      is_necessary,
    }
  }

  //   fn build_trait_errors(
  //     &self,
  //     obligations: &[Obligation<'tcx>],
  //   ) -> Vec<TraitError<'tcx>> {
  //     let tcx = &self.tcx;
  //     let source_map = tcx.sess.source_map();
  //     self
  //       .reported_trait_errors
  //       .borrow()
  //       .iter()
  //       .flat_map(|(span, predicates)| {
  //         let range = CharRange::from_span(*span, source_map)
  //           .expect("Couldn't get trait bound range");

  //         predicates.iter().map(move |predicate| {
  //           // TODO: these simple comparisons are not going to cut it ...
  //           // We can always take a similar approach to the ambiguity errors and
  //           // just recompute the errors that rustc does.
  //           //
  //           // Another idea would be to use the "X implies Y" mechanism from the
  //           // diagnostic system. This will collapse all implied errors into the one reported.
  //           let candidates = obligations
  //             .iter()
  //             .filter_map(|obl| {
  //               if !range.overlaps(obl.range) || obl.predicate != *predicate {
  //                 return None;
  //               }
  //               Some(obl.hash)
  //             })
  //             .collect::<Vec<_>>();

  //           TraitError {
  //             range,
  //             predicate: predicate.clone(),
  //             candidates,
  //           }
  //         })
  //       })
  //       .collect::<Vec<_>>()
  //   }

  //   fn build_ambiguity_errors(
  //     &self,
  //     _obligations: &[Obligation<'tcx>],
  //   ) -> Vec<AmbiguityError<'tcx>> {
  //     todo!()
  //   }
  // }
}

// impl<'tcx> FnCtxtExt<'tcx> for FnCtxt<'_, 'tcx> {
//   fn get_obligations(&self, ldef_id: LocalDefId) -> Vec<Obligation<'tcx>> {
//     todo!()
//     // let fulfilled = self.get_fulfilled(ldef_id);
//     // let (mut errors, _sucesses): (Vec<_>, Vec<_>) = fulfilled.into_iter()
//     //     .partition(FulfillmentData::is_error);
//     // self.adjust_fulfillment_errors_for_expr_obligation(&mut errors);
//     // self.convert_fulfilled(errors)
//   }

//   fn get_fulfilled(
//     &self,
//     ldef_id: LocalDefId,
//   ) -> Vec<FulfillmentData<'tcx>> {
//     todo!()

//     // let infcx = self.infcx().unwrap();

//     // let return_with_hashes = |v: Vec<(FulfillmentError<'tcx>, InferCtxt<'tcx>)>| {
//     //   self.tcx().with_stable_hashing_context(|mut hcx| {
//     //     v.into_iter()
//     //       .map(|(e, infcx)| {
//     //         FulfillmentData {
//     //           // NOTE: the shadowing of the infcx, is this actually what we want?
//     //           hash: e.stable_hash(&infcx, &mut hcx),
//     //           infcx,
//     //           data: FulfilledDataKind::Err(e)
//     //         }
//     //       })
//     //       .unique_by(FulfillmentData::get_hash)
//     //       .collect::<Vec<_>>()
//     //   })
//     // };

//     // // ------------------------------
//     // // NON-Updated code (with-probes)
//     // // TODO: once this is stable make sure this reduces
//     // // cloning (which it currently does not).
//     // let mut result = Vec::new();
//     // let _def_id = ldef_id.to_def_id();
//     // if let Some(infcx) = self.infcx() {
//     //   let fulfilled_obligations = infcx.fulfilled_obligations.borrow();
//     //   result.extend(fulfilled_obligations.iter().flat_map(|obl| match obl {
//     //     FulfilledObligation::Failed { data, infcx } => {
//     //       data.into_iter().map(|e| (e.clone(), infcx.clone())).collect::<Vec<_>>()

//     //     },
//     //     FulfilledObligation::Success { .. } => vec![],
//     //   }));
//     // }
//     // // ------------------------------

//     // let tcx = &self.tcx();
//     // // NOTE: this will remove everything that is not "necessary,"
//     // // below might be a better strategy. The best is ordering them by
//     // // relevance and then hiding unnecessary obligations unless the
//     // // user wants to see them.
//     // retain_fixpoint(&mut result, |(error, _)| {
//     //   error.obligation.predicate.is_necessary(tcx)
//     // });

//     // // Iteratively filter out elements unless there's only one thing
//     // // left; we don't want to remove the last remaining query.
//     // // Queries in order of *least* importance:
//     // // 1. (): TRAIT
//     // // 2. TY: Sized
//     // // 3. _: TRAIT
//     // // retain_fixpoint(&mut result, |error| {
//     // //   !error.obligation.predicate.is_unit_impl_trait(tcx)
//     // // });

//     // // retain_fixpoint(&mut result, |error| {
//     // //   !error.obligation.predicate.is_ty_impl_sized(tcx)
//     // // });

//     // // retain_fixpoint(&mut result, |error| {
//     // //   !error.obligation.predicate.is_ty_unknown(tcx)
//     // // });

//     // return_with_hashes(result)
//   }

//   fn convert_fulfilled(
//     &self,
//     errors: Vec<FulfillmentData<'tcx>>,
//   ) -> Vec<Obligation<'tcx>> {
//     todo!()
//   //   if errors.is_empty() {
//   //     return Vec::new();
//   //   }
//   //   let source_map = self.tcx().sess.source_map();
//   //   let _infcx = self.infcx().unwrap();

//   //   errors
//   //     .into_iter()
//   //     .map(|fdata| {
//   //       let predicate = fdata.get_obligation().predicate;
//   //       let range =
//   //         CharRange::from_span(fdata.get_cause_span(), source_map)
//   //           .unwrap();
//   //       Obligation {
//   //         predicate,
//   //         hash: fdata.get_hash().into(),
//   //         range,
//   //         kind: ObligationKind::Failure,
//   //       }
//   //     })
//   //     .collect::<Vec<_>>()
//   }
// }

// pub fn retain_fixpoint<T, F: FnMut(&T) -> bool>(v: &mut Vec<T>, mut pred: F) {
//   // NOTE: the original intent was to keep a single element, but that doesn't seem
//   // to be ideal. Perhaps it's best to remove all elements and then allow users to
//   // toggle these "hidden elements" should they choose to.
//   let keep_n_elems = 0;
//   let mut did_change = true;
//   let start_size = v.len();
//   let mut removed_es = 0usize;
//   // While things have changed, keep iterating, except
//   // when we have a single element left.
//   while did_change && start_size - removed_es > keep_n_elems {
//     did_change = false;
//     v.retain(|e| {
//       let r = pred(e);
//       did_change |= !r;
//       if !r && start_size - removed_es > keep_n_elems {
//         removed_es += 1;
//         r
//       } else {
//         true
//       }
//     });
//   }
// }

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
