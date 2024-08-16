use argus_ext::ty::TyCtxtExt;
use rustc_data_structures::fx::FxHashMap as HashMap;
use rustc_hir::{
  self as hir, intravisit::Visitor as HirVisitor, BodyId, HirId,
};
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

use crate::types::intermediate::ErrorAssemblyCtx;

pub fn associate_obligations_nodes(ctx: &ErrorAssemblyCtx) -> Vec<Bin> {
  let mut grouped: HashMap<_, Vec<_>> = HashMap::default();
  for (i, prov) in ctx.obligations.iter().enumerate() {
    grouped.entry(prov.hir_id).or_default().push(i);
  }

  bin_expressions(ctx, grouped)
}

// Given a map from [ HirId -> Vec< usize > ]
//
// we categorize obligations even further:
//
// * for function calls `frobnicate(arg1, arg2, ...)`
//
// -- obligations corresponding to `frobnicate`
// -- obligations corresponding call args `arg1, arg2, ...`, respectively.
// -- obligations corresponding to call `frobnicate(arg1, arg2, ...)`
//
// * for method calls `obj . frobnicate(arg1, arg2, ...)`
//
// -- obligations corresponding to `obj`
// -- obligations corresponding call args `arg1, arg2, ...`, respectively.
// -- obligations corresponding to `obj . frobnicate(arg1, arg2, ...)`
//
// TODO: for now, we will leave everything else *untouched*. We may want
// to expand this in the future, for example, when creating a system of
// traits it is important to debug why an impl block is invalid. (Either
// ill-formed, violates orphan rule, overlapping, etc...)
fn bin_expressions(
  ctx: &ErrorAssemblyCtx,
  mut map: HashMap<HirId, Vec<usize>>,
) -> Vec<Bin> {
  let mut binner = BinCreator {
    ctx,
    map: &mut map,
    bins: vec![],
  };

  binner.visit_body(ctx.tcx.hir().body(ctx.body_id));

  // Add remaining miscelanous unbinned obligations
  let mut bins = binner.bins;
  for (hir_id, obligations) in map {
    bins.push(Bin {
      hir_id,
      obligations,
      kind: BinKind::Misc,
    });
  }

  bins
}

#[derive(Debug)]
pub enum BinKind {
  CallableExpr,
  CallArg,
  Call,
  Misc,
}

pub struct Bin {
  pub hir_id: HirId,
  // TODO: use IndexVec for obligations instead of the--
  //
  // usize indexes into the obligation vec
  pub obligations: Vec<usize>,
  pub kind: BinKind,
}

struct BinCreator<'a, 'tcx: 'a> {
  ctx: &'a ErrorAssemblyCtx<'a, 'tcx>,
  map: &'a mut HashMap<HirId, Vec<usize>>,
  bins: Vec<Bin>,
}

impl BinCreator<'_, '_> {
  fn drain_nested(&mut self, target: HirId, kind: BinKind) {
    let is_nested = |id: HirId| self.ctx.tcx.is_parent_of(target, id);

    let obligations = self
      .map
      .extract_if(|&id, _| is_nested(id))
      .flat_map(|(_, idxs)| idxs)
      .collect::<Vec<_>>();

    if !obligations.is_empty() {
      log::debug!(
        "Associating obligations with {kind:?} {:?}\n{:#?}",
        self.ctx.tcx.hir().node_to_string(target),
        obligations
      );

      self.bins.push(Bin {
        hir_id: target,
        obligations,
        kind,
      });
    }
  }
}

impl<'a, 'tcx: 'a> HirVisitor<'_> for BinCreator<'a, 'tcx> {
  // FIXME: after updating to nightly-2024-05-20 this binning logic broke slightly.
  // Obligations associated with parameters are now being assigned to the overall call,
  // this makes more things use a method call table than necessary.
  fn visit_expr(&mut self, ex: &hir::Expr) {
    // Drain nested obligations first to match the most specific node possible.
    hir::intravisit::walk_expr(self, ex);

    match ex.kind {
      hir::ExprKind::Call(callable, args) => {
        for arg in args {
          self.drain_nested(arg.hir_id, BinKind::CallArg);
        }
        self.drain_nested(callable.hir_id, BinKind::CallableExpr);
        self.drain_nested(ex.hir_id, BinKind::Call);
      }
      hir::ExprKind::MethodCall(segment, func, args, _) => {
        for arg in args {
          self.drain_nested(arg.hir_id, BinKind::CallArg);
        }
        self.drain_nested(segment.hir_id, BinKind::Call);
        self.drain_nested(func.hir_id, BinKind::CallArg);
        // [ ] TODO (see above `FIXME`):
        // self.drain_nested(ex.hir_id, BinKind::MethodCall);
        self.drain_nested(ex.hir_id, BinKind::Misc);
      }
      _ => {}
    }
  }
}

// ------------------------------------------------

/// Find the `HirId` of the node that is the "most enclosing" of the span.
pub fn find_most_enclosing_node(
  tcx: TyCtxt,
  body_id: BodyId,
  span: Span,
) -> Option<HirId> {
  let hir = tcx.hir();
  let mut node_finder = FindNodeBySpan::new(span);
  node_finder.visit_body(hir.body(body_id));
  node_finder
    .result
    // NOTE: this should not happen because there must *at least* be an enclosing item.
    .map(|t| t.0)
}

/// Visitor for finding a `HirId` given a span.
///
/// Code taken from `rustc_trait_selection::traits::error_reporting` and modified
/// to find items that enclose the span, not just match it exactly.
struct FindNodeBySpan {
  pub span: Span,
  pub result: Option<(HirId, Span)>,
}

impl FindNodeBySpan {
  pub fn new(span: Span) -> Self {
    Self { span, result: None }
  }

  /// Is span `s` a closer match than the current best?
  fn is_better_match(&self, s: Span) -> bool {
    s.overlaps(self.span)
      && match self.result {
        None => true,
        Some((_, bsf)) => {
          let span = self.span.data();
          let bsf = bsf.data();
          let can = s.data();

          let dist = |outer: rustc_span::SpanData| {
            (outer.lo.max(span.lo) - outer.lo.min(span.lo))
              + (outer.hi.max(span.hi) - outer.hi.min(span.hi))
          };

          dist(can) < dist(bsf)
        }
      }
  }
}

macro_rules! simple_visitors {
  ( $( [$visitor:ident, $walker:ident, $t:ty], )* ) => {$(
      fn $visitor(&mut self, v: &$t) {
        hir::intravisit::$walker(self, v);
        if self.is_better_match(v.span) {
          self.result = Some((v.hir_id, v.span));
        }
      })*
  };
}

impl HirVisitor<'_> for FindNodeBySpan {
  simple_visitors! {
    [visit_param, walk_param, hir::Param],
    [visit_local, walk_local, hir::LetStmt],
    [visit_block, walk_block, hir::Block],
    [visit_stmt, walk_stmt, hir::Stmt],
    [visit_arm, walk_arm, hir::Arm],
    [visit_pat, walk_pat, hir::Pat],
    [visit_pat_field, walk_pat_field, hir::PatField],
    [visit_expr, walk_expr, hir::Expr],
    [visit_expr_field, walk_expr_field, hir::ExprField],
    [visit_ty, walk_ty, hir::Ty],
    [visit_generic_param, walk_generic_param, hir::GenericParam],
  }
}
