//! ProofTree analysis.
use std::{
  fmt::{self, Debug, Formatter},
  hash::{Hash, Hasher},
  ops::Deref,
};

use anyhow::{anyhow, Result};
use fluid_let::fluid_let;
use rustc_hir::{def_id::LocalDefId, hir_id::HirId, BodyId};
use rustc_infer::{
  infer::InferCtxt,
  traits::{ObligationInspector, PredicateObligation},
};
use rustc_middle::ty::TyCtxt;
use rustc_trait_selection::traits::solve::Goal;
use rustc_utils::source_map::range::CharRange;

mod ambiguous;
mod entry;
mod tls;

pub(crate) use crate::types::intermediate::{
  EvaluationResult, FulfillmentData,
};
use crate::{
  ext::{InferCtxtExt, TyCtxtExt},
  proof_tree::serialize::serialize_proof_tree,
  serialize::serialize_to_value,
  types::{ObligationsInBody, Target},
};

fluid_let! {
  pub static OBLIGATION_TARGET: Target;
}

pub fn obligations<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
) -> Result<serde_json::Value> {
  let typeck_results = tcx.inspect_typeck(body_id, entry::process_obligation);

  // Construct the output from the stored data.
  entry::build_obligations_output(tcx, body_id, typeck_results)
}

// NOTE: tree is only invoked for *a single* tree, it must be found
// within the `body_id` and the appropriate `OBLIGATION_TARGET` (i.e., stable hash).
pub fn tree<'tcx>(
  tcx: TyCtxt<'tcx>,
  body_id: BodyId,
) -> Result<serde_json::Value> {
  tcx.inspect_typeck(body_id, entry::process_obligation_for_tree);
  entry::build_tree_output(tcx, body_id).ok_or_else(|| {
    OBLIGATION_TARGET.get(|target| {
      anyhow!("failed to locate proof tree with target {:?}", target)
    })
  })
}

// ------------------------------------------------------------

// The provenance about where an element came from,
// or was "spawned from," in the HIR. This type is intermediate
// but stored in the TLS, it shouldn't capture lifetimes but
// can capture unstable hashes.
pub(crate) struct Provenance<T: Sized> {
  // The expression from whence `it` came, the
  // referenced element is expected to be an
  // expression.
  originating_expression: HirId,

  it: T,
}

impl<T: Sized> Deref for Provenance<T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    &self.it
  }
}

impl<T: Sized> Provenance<T> {
  pub(super) fn forget(self) -> T {
    self.it
  }

  pub(super) fn is_nested<U: Sized>(&self, other: Provenance<U>) -> bool {
    todo!()
  }
}

impl<T: Debug> Debug for Provenance<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "Provenance<{:?}>", self.it)
  }
}

impl<T: PartialEq> PartialEq for Provenance<T> {
  fn eq(&self, other: &Self) -> bool {
    self.it == other.it
  }
}

impl<T: Eq> Eq for Provenance<T> {}

impl<T: Hash> Hash for Provenance<T> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.it.hash(state)
  }
}
