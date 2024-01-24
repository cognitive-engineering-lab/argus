use rustc_hir::{self as hir, intravisit::Visitor, BodyId, HirId};
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

/// Find the HirId of the item that is the "most enclosing" of the span.
///
/// This should favor expressions, and in the worst case will return the body id.
pub fn find_most_enclosing_node<'tcx>(
  tcx: &TyCtxt<'tcx>,
  body_id: BodyId,
  span: Span,
) -> Option<HirId> {
  let hir = tcx.hir();
  let mut expr_finder = FindExprBySpan::new(*tcx, span);
  expr_finder.visit_expr(hir.body(body_id).value);
  expr_finder
    .result
    // NOTE: this should not happen because there must *at least* be an enclosing item.
    .map(|t| t.0)
}

// NOTE: this probably needs to be expanded to account for all nodes, not just expressions.
struct FindExprBySpan<'tcx> {
  tcx: TyCtxt<'tcx>,
  pub span: Span,
  pub result: Option<(HirId, Span)>,
}

// Code taken from rustc_trait_selection::traits::error_reporting,
// modified to find items that enclose the span, not just match it
// exactly.
// TODO: this should work on all nodes, not just expressions.
impl<'tcx> FindExprBySpan<'tcx> {
  pub fn new(tcx: TyCtxt<'tcx>, span: Span) -> Self {
    Self {
      tcx,
      span,
      result: None,
    }
  }
}

impl FindExprBySpan<'_> {
  fn is_better_match(&self, s: Span) -> bool {
    s.contains(self.span)
      && match self.result {
        None => true,
        Some((_, bsf)) => {
          let span = self.span.data();
          let bsf = bsf.data();
          let can = s.data();

          let dist = |outer: rustc_span::SpanData| {
            debug_assert!(outer.lo <= span.lo && span.hi <= outer.hi);
            span.lo - outer.lo + outer.hi - span.hi
          };

          dist(can) < dist(bsf)
        }
      }
  }
}

impl<'v> Visitor<'v> for FindExprBySpan<'v> {
  fn visit_expr(&mut self, ex: &'v hir::Expr<'v>) {
    if self.is_better_match(ex.span) {
      self.result = Some((ex.hir_id, ex.span));
    }

    hir::intravisit::walk_expr(self, ex);
  }
}

trait NodeExt {
  fn span(&self) -> Span;
}

impl NodeExt for hir::Node<'_> {
  fn span(&self) -> Span {
    use rustc_hir::Node::*;
    match &self {
      Param(v) => v.span,
      Item(v) => v.span,
      ForeignItem(v) => v.span,
      TraitItem(v) => v.span,
      ImplItem(v) => v.span,
      Variant(v) => v.span,
      Field(v) => v.span,
      Expr(v) => v.span,
      ExprField(v) => v.span,
      Stmt(v) => v.span,
      Ty(v) => v.span,
      TypeBinding(v) => v.span,
      Pat(v) => v.span,
      PatField(v) => v.span,
      Arm(v) => v.span,
      Block(v) => v.span,
      Local(v) => v.span,
      GenericParam(v) => v.span,
      Infer(v) => v.span,

      // items without a direct span field
      _ => panic!(),
      // Ctor(v) => v.span,
      // TraitRef(v) => v.span,
      // Lifetime(v) => v.span,
      // Crate(v) => v.span,
      // PathSegment(v) => v.span,
      // AnonConst(v) => v.span,
      // ConstBlock(v) => v.span,
    }
  }
}
