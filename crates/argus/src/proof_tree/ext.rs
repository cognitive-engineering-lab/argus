use rustc_hir::{def::Namespace, def_id::DefId, definitions::DefPathData};
use rustc_infer::infer::InferCtxt;
use rustc_middle::ty::{
  print::{FmtPrinter, Print},
  TyCtxt,
};
use rustc_trait_selection::{
  solve::inspect::InspectCandidate,
  traits::{
    query::NoSolution,
    solve::{inspect::ProbeKind, CandidateSource, Certainty, MaybeCause},
  },
};

/// Pretty printing for results.
pub trait PrettyResultExt {
  fn pretty(&self) -> String;
  fn is_yes(&self) -> bool;
}

/// Pretty printing for `Candidates`.
pub trait CandidateExt {
  fn pretty(&self, infcx: &InferCtxt, def_id: DefId) -> String;

  fn is_informative_probe(&self) -> bool;
}

// -----------------------------------------------
// Impls

// impl<'a, 'tcx, T: Print<'tcx, FmtPrinter<'a, 'tcx>>> PrettyPrintExt<'a, 'tcx>
//   for T
// {
//   /* intentionally blank */
// }

/// Pretty printer for results
impl PrettyResultExt for Result<Certainty, NoSolution> {
  fn is_yes(&self) -> bool {
    matches!(self, Ok(Certainty::Yes))
  }

  fn pretty(&self) -> String {
    let str = match self {
      Ok(Certainty::Yes) => "Yes",
      Ok(Certainty::Maybe(MaybeCause::Overflow)) => "No: Overflow",
      Ok(Certainty::Maybe(MaybeCause::Ambiguity)) => "No: Ambiguity",
      Err(NoSolution) => "No",
    };

    str.to_string()
  }
}

impl CandidateExt for InspectCandidate<'_, '_> {
  fn pretty(&self, _infcx: &InferCtxt, _def_id: DefId) -> String {
    match self.kind() {
      ProbeKind::Root { .. } => "root".to_string(),
      ProbeKind::NormalizedSelfTyAssembly => {
        "normalized-self-ty-asm".to_string()
      }
      ProbeKind::UnsizeAssembly => "unsize-asm".to_string(),
      ProbeKind::CommitIfOk => "commit-if-ok".to_string(),
      ProbeKind::UpcastProjectionCompatibility => {
        "upcase-proj-compat".to_string()
      }
      ProbeKind::MiscCandidate { name, .. } => format!("misc-{}", name),
      ProbeKind::TraitCandidate { source, .. } => match source {
        CandidateSource::BuiltinImpl(_built_impl) => "builtin".to_string(),
        CandidateSource::AliasBound => "alias-bound".to_string(),

        // The only two we really care about.
        CandidateSource::ParamEnv(idx) => format!("param-env#{idx}"),
        CandidateSource::Impl(_def_id) => "impl".to_string(),
      },
    }
  }

  fn is_informative_probe(&self) -> bool {
    matches!(
      self.kind(),
      ProbeKind::TraitCandidate {
        source: CandidateSource::Impl(_),
        ..
      } | ProbeKind::TraitCandidate {
        source: CandidateSource::BuiltinImpl(_),
        ..
      }
    )
  }
}
