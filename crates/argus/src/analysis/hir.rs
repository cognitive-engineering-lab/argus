

// Write a function, given a TyCtxt, and FulfillmentData, that computes the most enclosing expression.
fn get_enclosing_expression<'tcx>(
  tcx: TyCtxt<'tcx>,
  fdata: &FulfillmentData<'tcx>,
) -> Option<HirId> {
  let mut enclosing_expression = None;

  for (ty, _) in fdata.obligations() {
    let ty = tcx.erase_regions(ty);
    let ty = tcx.lift_to_global(&ty)?;

    let hir_id = tcx.hir().local_def_id_to_hir_id(ty.def_id)?;

    let enclosing = enclosing_expression.take();

    enclosing_expression = Some(Provenance {
      originating_expression: hir_id,
      it: enclosing,
    });
  }

  enclosing_expression
}