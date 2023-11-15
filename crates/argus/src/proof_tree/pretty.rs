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

use rustc_trait_selection::traits::{
    solve::{Certainty, MaybeCause},
    query::NoSolution,
};

pub trait PrettyResultExt {
    fn pretty(&self) -> String;
    fn is_yes(&self) -> bool;
}

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

pub trait PrettyPrintExt<'a, 'tcx>: Print<'tcx, FmtPrinter<'a, 'tcx>> {
    fn pretty(&self, infcx: &'a InferCtxt<'tcx>, def_id: DefId) -> String {
        let tcx = infcx.tcx;
        let namespace = guess_def_namespace(tcx, def_id);
        let mut fmt = FmtPrinter::new(tcx, namespace);
        self.print(&mut fmt);
        fmt.into_buffer()
    }
}

impl<'a, 'tcx, T: Print<'tcx, FmtPrinter<'a, 'tcx>>> PrettyPrintExt<'a, 'tcx> for T {}

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
