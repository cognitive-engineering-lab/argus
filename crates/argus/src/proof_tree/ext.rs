use rustc_data_structures::stable_hasher::{Hash64, HashStable, StableHasher};
use rustc_hir::{
  def::Namespace, def_id::DefId, definitions::DefPathData, LangItem,
};
use rustc_infer::infer::InferCtxt;
use rustc_middle::ty::{
  self,
  print::{FmtPrinter, Print},
  Ty, TyCtxt, TypeFoldable, TypeFolder, TypeSuperFoldable,
};
use rustc_query_system::ich::StableHashingContext;
use rustc_trait_selection::{
  solve::inspect::InspectCandidate,
  traits::{
    query::NoSolution,
    solve::{inspect::ProbeKind, CandidateSource, Certainty, MaybeCause},
    FulfillmentError,
  },
};

/// Pretty printing for things that can already be printed.
pub trait PrettyPrintExt<'a, 'tcx>: Print<'tcx, FmtPrinter<'a, 'tcx>> {
  fn pretty(&self, infcx: &'a InferCtxt<'tcx>, def_id: DefId) -> String {
    let tcx = infcx.tcx;
    let namespace = guess_def_namespace(tcx, def_id);
    let mut fmt = FmtPrinter::new(tcx, namespace);
    self.print(&mut fmt).unwrap();
    fmt.into_buffer()
  }
}

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

impl<'a, 'tcx, T: Print<'tcx, FmtPrinter<'a, 'tcx>>> PrettyPrintExt<'a, 'tcx>
  for T
{
  /* intentionally blank */
}

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
    // TODO: gavinleroy
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

// -----------------------------------------------
// Helpers

fn guess_def_namespace(tcx: TyCtxt<'_>, def_id: DefId) -> Namespace {
  match tcx.def_key(def_id).disambiguated_data.data {
    DefPathData::TypeNs(..)
    | DefPathData::CrateRoot
    | DefPathData::OpaqueTy => Namespace::TypeNS,

    DefPathData::ValueNs(..)
    | DefPathData::AnonConst
    | DefPathData::Closure
    | DefPathData::Ctor => Namespace::ValueNS,

    DefPathData::MacroNs(..) => Namespace::MacroNS,

    _ => Namespace::TypeNS,
  }
}

