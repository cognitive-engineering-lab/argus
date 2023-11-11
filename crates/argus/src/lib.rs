#![feature(rustc_private)]

extern crate rustc_hir;
extern crate rustc_infer;
extern crate rustc_middle;
extern crate rustc_trait_selection;

// --- move elsewhere ---

use rustc_hir::BodyId;
use rustc_middle::ty::TyCtxt;
use rustc_trait_selection::{infer::TyCtxtInferExt, traits::ObligationCtxt};

pub fn trees_in_body(tcx: &TyCtxt<'_>, body_id: BodyId) {
  let _ = tcx.typeck_body(body_id);

  let ifcx = tcx.infer_ctxt().with_next_trait_solver(true).build();

  let obcx = ObligationCtxt::new(&ifcx);

  let errs = obcx.select_all_or_error();

  log::debug!("select_all_or_error {:#?}", errs);
}
