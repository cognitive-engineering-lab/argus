use std::fmt::Write;

use rustc_infer::infer::InferCtxt;
use rustc_middle::ty::Predicate;
use rustc_span::Span;
use rustc_trait_selection::{
  solve::inspect::{
    InspectCandidate, InspectGoal, ProofTreeInferCtxtExt, ProofTreeVisitor,
  },
  traits::solve,
};

pub fn dump_proof_tree<'tcx>(
  goal: solve::Goal<'tcx, Predicate<'tcx>>,
  span: Span,
  infcx: &InferCtxt<'tcx>,
) {
  let do_format = move |f: &mut std::fmt::Formatter<'_>| {
    let mut fm = ProofTreeFormatter::new(f, span);
    infcx.visit_proof_tree(goal, &mut fm);
    Ok(())
  };

  log::debug!("TREE DUMP\nFor {:?}\n{:?}", goal, Formatter(&do_format));
}

struct Formatter<'a>(
  &'a dyn Fn(&mut std::fmt::Formatter<'_>) -> std::fmt::Result,
);

impl std::fmt::Debug for Formatter<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    (self.0)(f)
  }
}

pub struct ProofTreeFormatter<'a, 'b> {
  f: &'a mut (dyn Write + 'b),
  span: Span,
}

enum IndentorState {
  StartWithNewline,
  OnNewline,
  Inline,
}

/// A formatter which adds 4 spaces of indentation to its input before
/// passing it on to its nested formatter.
///
/// We can use this for arbitrary levels of indentation by nesting it.
struct Indentor<'a, 'b> {
  f: &'a mut (dyn Write + 'b),
  state: IndentorState,
}

impl Write for Indentor<'_, '_> {
  fn write_str(&mut self, s: &str) -> std::fmt::Result {
    for line in s.split_inclusive('\n') {
      match self.state {
        IndentorState::StartWithNewline => self.f.write_str("\n    ")?,
        IndentorState::OnNewline => self.f.write_str("    ")?,
        IndentorState::Inline => {}
      }
      self.state = if line.ends_with('\n') {
        IndentorState::OnNewline
      } else {
        IndentorState::Inline
      };
      self.f.write_str(line)?;
    }

    Ok(())
  }
}

impl<'a, 'b> ProofTreeFormatter<'a, 'b> {
  pub(super) fn new(f: &'a mut (dyn Write + 'b), span: Span) -> Self {
    ProofTreeFormatter { f, span }
  }

  fn nested<F>(&mut self, func: F) -> std::fmt::Result
  where
    F: FnOnce(&mut ProofTreeFormatter<'_, '_>) -> std::fmt::Result,
  {
    write!(self.f, " {{")?;
    func(&mut ProofTreeFormatter {
      f: &mut Indentor {
        f: self.f,
        state: IndentorState::StartWithNewline,
      },
      span: self.span,
    })?;
    writeln!(self.f, "}}")
  }

  fn format_goal(&mut self, goal: &InspectGoal<'_, '_>) -> std::fmt::Result {
    write!(self.f, "GOAL: {:?}", goal.goal())?;
    let candidates = goal.candidates();
    write!(self.f, "\n({} candidates)", candidates.len())?;
    self.nested(move |this| {
      for (i, can) in candidates.into_iter().enumerate() {
        write!(this.f, "CANDIDATE {i}: ")?;
        this.format_candidate(&can)?;
      }
      Ok(())
    })
  }

  fn format_candidate(
    &mut self,
    candidate: &InspectCandidate,
  ) -> std::fmt::Result {
    write!(self.f, "{:?}", candidate.kind())?;
    self.nested(|this| {
      candidate.visit_nested_in_probe(this);
      Ok(())
    })
  }
}

impl<'tcx> ProofTreeVisitor<'tcx> for ProofTreeFormatter<'_, '_> {
  type Result = ();

  fn span(&self) -> Span {
    self.span
  }

  fn visit_goal(&mut self, goal: &InspectGoal<'_, 'tcx>) -> Self::Result {
    self.format_goal(goal).unwrap();
  }
}
