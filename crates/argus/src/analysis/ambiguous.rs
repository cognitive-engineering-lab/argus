//! Analysis for ambiguous method calls.
//!
//! This file "simulates" what `rustc_hir_typeck` does for type-checking
//! method call expressions. Only, that we want to keep around *a lot*
//! more information.
use rustc_hir::{self as hir, HirId};
use rustc_middle::ty::{TyCtxt, TypeckResults};

use crate::ext::{TyCtxtExt, TypeckResultsExt};

pub struct AmbigMethodLookupData<'tcx> {
  /// Expr of the overall method call `obj.frobnicate(...)`
  pub expr: &'tcx hir::Expr<'tcx>,

  /// Expr of the receiver `obj`.
  pub recvr: &'tcx hir::Expr<'tcx>,

  /// Method call segment.
  pub segment: &'tcx hir::PathSegment<'tcx>,

  /// Method call arguments.
  pub args: &'tcx [hir::Expr<'tcx>],
}

// NOTE: we cannot use the `TypeckResults::is_method_call` because it
// check the `type_dependent_defs` table which *does not have* an entry
// for unresolved method calls.
pub fn get_ambiguous_trait_method_exprs<'tcx>(
  tcx: &TyCtxt<'tcx>,
  typeck_results: &TypeckResults<'tcx>,
) -> Vec<AmbigMethodLookupData<'tcx>> {
  let hir = tcx.hir();
  typeck_results
    .error_nodes()
    .filter_map(|hir_id| {
      let expr = hir.expect_expr(hir_id);
      gather_ambig_data(tcx, typeck_results, expr)
    })
    .collect::<Vec<_>>()
}

pub fn gather_ambig_data<'tcx>(
  tcx: &TyCtxt<'tcx>,
  typeck_results: &TypeckResults<'tcx>,
  expr: &'tcx hir::Expr<'tcx>,
) -> Option<AmbigMethodLookupData<'tcx>> {
  let hir::ExprKind::MethodCall(segment, recvr, args, span) = &expr.kind else {
    return None;
  };

  // TODO: as more data is needed

  Some(AmbigMethodLookupData {
    expr,
    recvr,
    segment,
    args,
  })
}
