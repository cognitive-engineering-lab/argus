use argus_ext::{
  infer::InferCtxtExt as ArgusInferCtxtExt,
  ty::{PredicateExt, PredicateObligationExt},
};
use argus_ser as ser;
use rustc_hir::BodyId;
use rustc_infer::{infer::InferCtxt, traits::PredicateObligation};
use rustc_middle::ty::{self, Predicate};
use serde::Serialize;

use crate::{
  analysis::{EvaluationResult, FulfillmentData},
  types::{Obligation, ObligationNecessity},
};

pub trait InferCtxtExt<'tcx> {
  fn bless_fulfilled<'a>(
    &self,
    obligation: &'a PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> FulfillmentData<'a, 'tcx>;

  fn erase_non_local_data(
    &self,
    body_id: BodyId,
    fdata: FulfillmentData<'_, 'tcx>,
  ) -> Obligation;

  fn guess_predicate_necessity(
    &self,
    p: &Predicate<'tcx>,
  ) -> ObligationNecessity;

  fn obligation_necessity(
    &self,
    obligation: &PredicateObligation<'tcx>,
  ) -> ObligationNecessity;
}

impl<'tcx> InferCtxtExt<'tcx> for InferCtxt<'tcx> {
  fn guess_predicate_necessity(
    &self,
    p: &Predicate<'tcx>,
  ) -> ObligationNecessity {
    use ObligationNecessity as ON;

    let is_rhs_lang_item = || {
      self
        .tcx
        .lang_items()
        .iter()
        .any(|(_lang_item, def_id)| p.is_trait_pred_rhs(def_id))
    };

    let is_writeable = || {
      use ty::ClauseKind as CK;
      matches!(
        p.kind().skip_binder(),
        ty::PredicateKind::Clause(
          CK::Trait(..)
            | CK::RegionOutlives(..)
            | CK::TypeOutlives(..)
            | CK::Projection(..)
        )
      )
    };

    if !is_writeable() || p.is_lhs_unit() {
      ON::No
    } else if (p.is_trait_predicate() && is_rhs_lang_item())
      || !p.is_trait_predicate()
    {
      ON::OnError
    } else {
      ON::Yes
    }
  }

  /// Determine what level of "necessity" an obligation has.
  ///
  /// For example, obligations with a cause `SizedReturnType`,
  /// with a `self_ty` `()` (unit), is *unecessary*. Obligations whose
  /// kind is not a Trait Clause, are generally deemed `ForProfessionals`
  /// (that is, you can get them when interested), and others are shown
  /// `OnError`. Necessary obligations are trait predicates where the
  /// type and trait are not `LangItems`.
  fn obligation_necessity(
    &self,
    obligation: &PredicateObligation<'tcx>,
  ) -> ObligationNecessity {
    use rustc_infer::traits::ObligationCauseCode;
    use ObligationNecessity as ON;

    let p = &obligation.predicate;
    let code = obligation.cause.code();

    if matches!(code, ObligationCauseCode::SizedReturnType) && p.is_lhs_unit()
      || matches!(p.as_trait_predicate(), Some(p) if p.self_ty().skip_binder().is_ty_var())
    {
      ON::No
    } else {
      self.guess_predicate_necessity(p)
    }
  }

  fn bless_fulfilled<'a>(
    &self,
    obligation: &'a PredicateObligation<'tcx>,
    result: EvaluationResult,
  ) -> FulfillmentData<'a, 'tcx> {
    FulfillmentData {
      hash: self.predicate_hash(&obligation.predicate).into(),
      obligation,
      result,
    }
  }

  fn erase_non_local_data(
    &self,
    body_id: BodyId,
    fdata: FulfillmentData<'_, 'tcx>,
  ) -> Obligation {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Wrapper<'a, 'tcx: 'a>(
      #[serde(with = "ser::ty::PredicateObligationDef")]
      &'a PredicateObligation<'tcx>,
    );

    let obl = &fdata.obligation;
    let range = obl.range(&self.tcx, body_id);
    let necessity = self.obligation_necessity(obl);
    let obligation = crate::tls::unsafe_access_interner(|ty_intern| {
      ser::to_value_expect(self, ty_intern, &Wrapper(obl))
    });

    Obligation {
      obligation,
      hash: fdata.hash,
      range,
      kind: fdata.kind(),
      necessity,
      result: fdata.result,
    }
  }
}
