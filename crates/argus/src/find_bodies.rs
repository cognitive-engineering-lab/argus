//! This is a copy of the `BodyFinder` from `rustc_utils` but it
//! does *not* skip const/static items. Funny enough, these items
//! often have important trait constraints evaluated (think derive macros).
use rustc_hir::{intravisit::Visitor, BodyId};
use rustc_middle::{hir::nested_filter::OnlyBodies, ty::TyCtxt};
use rustc_span::Span;
use rustc_utils::{block_timer, SpanExt};

struct BodyFinder<'tcx> {
  tcx: TyCtxt<'tcx>,
  bodies: Vec<(Span, BodyId)>,
}

impl<'tcx> Visitor<'tcx> for BodyFinder<'tcx> {
  type NestedFilter = OnlyBodies;

  fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
    self.tcx
  }

  fn visit_nested_body(&mut self, id: BodyId) {
    // // const/static items are considered to have bodies, so we want to exclude
    // // them from our search for functions
    // if !hir
    //   .body_owner_kind(hir.body_owner_def_id(id))
    //   .is_fn_or_closure()
    // {
    //   return;
    // }

    let body = self.tcx.hir_body(id);
    self.visit_body(body);

    let hir = self.tcx.hir();
    let span = hir.span_with_body(self.tcx.hir_body_owner(id));
    log::trace!(
      "Searching body for {:?} with span {span:?} (local {:?})",
      self
        .tcx
        .def_path_debug_str(self.tcx.hir_body_owner_def_id(id).to_def_id()),
      span.as_local(body.value.span)
    );

    self.bodies.push((span, id));
  }
}

/// Finds all bodies in the current crate
pub fn find_bodies(tcx: TyCtxt) -> Vec<(Span, BodyId)> {
  block_timer!("find_bodies");
  let mut finder = BodyFinder {
    tcx,
    bodies: Vec::new(),
  };
  tcx.hir_visit_all_item_likes_in_crate(&mut finder);
  finder.bodies
}

/// Finds all the bodies that enclose the given span, from innermost to outermost
pub fn find_enclosing_bodies(
  tcx: TyCtxt,
  sp: Span,
) -> impl Iterator<Item = BodyId> {
  let mut bodies = find_bodies(tcx);
  bodies.retain(|(other, _)| other.contains(sp));
  bodies.sort_by_key(|(span, _)| span.size());
  bodies.into_iter().map(|(_, id)| id)
}
