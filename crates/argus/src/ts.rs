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

#[test]
fn export_bindings_indices() {
  ts! {
    ty::ExprIdx,
    ty::MethodLookupIdx,
    ty::ObligationIdx,

    pty::ProofNodeIdx,
    pty::GoalIdx,
    pty::CandidateIdx,
    pty::ResultIdx,
  }
}

#[test]
fn export_bindings_rustc_utils() {
  ts! {
    uty::CharPos,
    uty::CharRange,
    fty::FilenameIndex,
  }
}
