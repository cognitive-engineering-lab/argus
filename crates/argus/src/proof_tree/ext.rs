use rustc_middle::ty::print::{FmtPrinter, Print, PrettyPrinter};
use rustc_middle::mir::interpret::{AllocRange, GlobalAlloc, Pointer, Provenance, Scalar};
use rustc_middle::query::IntoQueryParam;
use rustc_middle::query::Providers;
use rustc_middle::traits::util::supertraits_for_pretty_printing;
use rustc_middle::ty::{
    self, ConstInt, ParamConst, ScalarInt, Term, TermKind, Ty, TyCtxt, TypeFoldable,
    TypeSuperFoldable, TypeSuperVisitable, TypeVisitable, TypeVisitableExt,
};
use rustc_middle::ty::{GenericArg, GenericArgKind};
use rustc_hir::def::{self, CtorKind, DefKind, Namespace};
use rustc_hir::def_id::{DefId, DefIdSet, ModDefId, CRATE_DEF_ID, LOCAL_CRATE};
use rustc_hir::definitions::{DefKey, DefPathData, DefPathDataName, DisambiguatedDefPathData};
use rustc_hir::LangItem;
use rustc_infer::infer::InferCtxt;

use rustc_trait_selection::solve::inspect::InspectCandidate;
use rustc_trait_selection::traits::solve::{Certainty, MaybeCause};
use rustc_trait_selection::traits::query::NoSolution;
use rustc_trait_selection::traits::solve::{inspect::ProbeKind, CandidateSource};

pub trait PredicateExt<'tcx> {
    fn is_necessary(&self, tcx: &TyCtxt<'tcx>) -> bool;
}

/// Pretty printing for things that can already be printed.
pub trait PrettyPrintExt<'a, 'tcx>: Print<'tcx, FmtPrinter<'a, 'tcx>> {
    fn pretty(&self, infcx: &'a InferCtxt<'tcx>, def_id: DefId) -> String {
        let tcx = infcx.tcx;
        let namespace = guess_def_namespace(tcx, def_id);
        let mut fmt = FmtPrinter::new(tcx, namespace);
        self.print(&mut fmt);
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


impl<'tcx> PredicateExt<'tcx> for ty::Predicate<'tcx> {
    fn is_necessary(&self, tcx: &TyCtxt<'tcx>) -> bool {
        matches!(self.kind().skip_binder(), ty::PredicateKind::Clause(ty::ClauseKind::Trait(trait_predicate)) if {
            // NOTE: predicates of the form `_: TRAIT` and `(): TRAIT` are useless. The first doesn't have
            // any information about the type of the Self var, and I've never understood why the latter
            // occurs so frequently.
            !trait_predicate.self_ty().is_ty_var() &&
            !trait_predicate.self_ty().is_unit() &&
                trait_predicate.def_id() != tcx.require_lang_item(LangItem::Sized, None)
        })
    }
}

impl<'a, 'tcx, T: Print<'tcx, FmtPrinter<'a, 'tcx>>> PrettyPrintExt<'a, 'tcx> for T {}

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
            Err(NoSolution) => "No"
        };

        str.to_string()
    }
}

impl CandidateExt for InspectCandidate<'_, '_> {
    fn pretty(&self, infcx: &InferCtxt, def_id: DefId) -> String {
        // TODO: gavinleroy
        match self.kind() {
            ProbeKind::Root { .. } => "root".to_string(),
            ProbeKind::NormalizedSelfTyAssembly => "normalized-self-ty-asm".to_string(),
            ProbeKind::UnsizeAssembly => "unsize-asm".to_string(),
            ProbeKind::CommitIfOk => "commit-if-ok".to_string(),
            ProbeKind::UpcastProjectionCompatibility => "upcase-proj-compat".to_string(),
            ProbeKind::MiscCandidate {
                name,
                ..
            } => format!("misc-{}", name),
            ProbeKind::TraitCandidate {
                source,
                ..
            } => match source {
                CandidateSource::BuiltinImpl(built_impl) => "builtin".to_string(),
                CandidateSource::AliasBound => "alias-bound".to_string(),

                // The only two we really care about.
                CandidateSource::ParamEnv(idx) => format!("param-env#{idx}"),
                CandidateSource::Impl(def_id) => "impl".to_string(),
            },
        }
    }

    fn is_informative_probe(&self) -> bool {
        matches!(self.kind(), ProbeKind::TraitCandidate {
            source: CandidateSource::Impl(_),
            ..
        } | ProbeKind::TraitCandidate {
            source: CandidateSource::BuiltinImpl(_),
            ..
        })
    }
}

// -----------------------------------------------
// Helpers

fn guess_def_namespace(tcx: TyCtxt<'_>, def_id: DefId) -> Namespace {
    match tcx.def_key(def_id).disambiguated_data.data {
        DefPathData::TypeNs(..) | DefPathData::CrateRoot | DefPathData::ImplTrait => {
            Namespace::TypeNS
        }

        DefPathData::ValueNs(..)
        | DefPathData::AnonConst
        | DefPathData::ClosureExpr
        | DefPathData::Ctor => Namespace::ValueNS,

        DefPathData::MacroNs(..) => Namespace::MacroNS,

        _ => Namespace::TypeNS,
    }
}
