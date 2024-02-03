use rustc_utils::source_map::{filename as fty, range as uty};

use crate::{proof_tree as pty, types as ty};

macro_rules! ts {
  ($($ty:ty,)*) => {
    $({
      let error_msg = format!("Failed to export TS binding for type '{}'", stringify!($ty));
      <$ty as ts_rs::TS>::export().expect(error_msg.as_ref());
    })*
  };
}

// Legend
// mod `ty`: types from argus
// mod `pty`: types from argus::proof_tree
// mod `(f|u)ty`: types from rustc_utils
#[test]
fn export_bindings_all_tys() {
  ts! {
      uty::CharPos,
      uty::CharRange,
      fty::FilenameIndex,

  ty::ObligationIdx,

      pty::Node,
      pty::Candidate,
      pty::Goal,
      pty::SerializedTree,
      pty::TreeTopology,
      pty::ProofNodeIdx,

      ty::Expr,
      ty::ExprIdx,
      ty::ExprKind,
      ty::MethodLookup,
      ty::MethodLookupIdx,
      ty::MethodStep,
      ty::Obligation,
      ty::ObligationHash,

      ty::ObligationKind,
      ty::ObligationNecessity,
      ty::ObligationsInBody,
      ty::ReceiverAdjStep,
    }
}
