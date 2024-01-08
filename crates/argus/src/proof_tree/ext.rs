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

pub trait StableHash<'__ctx, 'tcx>:
  HashStable<StableHashingContext<'__ctx>>
{
  fn stable_hash(
    self,
    infcx: &InferCtxt<'tcx>,
    ctx: &mut StableHashingContext<'__ctx>,
  ) -> Hash64;
}

pub trait PredicateExt<'tcx> {
  fn is_unit_impl_trait(&self, tcx: &TyCtxt<'tcx>) -> bool;
  fn is_ty_impl_sized(&self, tcx: &TyCtxt<'tcx>) -> bool;
  fn is_ty_unknown(&self, tcx: &TyCtxt<'tcx>) -> bool;
  fn is_trait_predicate(&self) -> bool;
  fn is_necessary(&self, tcx: &TyCtxt<'tcx>) -> bool;
}

pub trait FulfillmentErrorExt<'tcx> {
  fn stable_hash(
    &self,
    infcx: &InferCtxt<'tcx>,
    ctx: &mut StableHashingContext,
  ) -> Hash64;
}

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

impl<'__ctx, 'tcx, T> StableHash<'__ctx, 'tcx> for T
where
  T: HashStable<StableHashingContext<'__ctx>>,
  T: TypeFoldable<TyCtxt<'tcx>>,
{
  fn stable_hash(
    self,
    infcx: &InferCtxt<'tcx>,
    ctx: &mut StableHashingContext<'__ctx>,
  ) -> Hash64 {
    let mut h = StableHasher::new();
    let sans_regions = infcx.tcx.erase_regions(self);
    let this = sans_regions.fold_with(&mut TyVarEraserVisitor { infcx });
    // erase infer vars
    this.hash_stable(ctx, &mut h);
    h.finish()
  }
}

impl<'tcx> FulfillmentErrorExt<'tcx> for FulfillmentError<'tcx> {
  fn stable_hash(
    &self,
    infcx: &InferCtxt<'tcx>,
    ctx: &mut StableHashingContext,
  ) -> Hash64 {
    // FIXME: should we be using the root_obligation here?
    // The issue is that type variables cannot use hash_stable.
    self.root_obligation.predicate.stable_hash(infcx, ctx)
  }
}

impl<'tcx> PredicateExt<'tcx> for ty::Predicate<'tcx> {
  fn is_unit_impl_trait(&self, _tcx: &TyCtxt<'tcx>) -> bool {
    matches!(self.kind().skip_binder(),
    ty::PredicateKind::Clause(ty::ClauseKind::Trait(trait_predicate)) if {
        trait_predicate.self_ty().is_unit()
    })
  }

  fn is_ty_impl_sized(&self, tcx: &TyCtxt<'tcx>) -> bool {
    matches!(self.kind().skip_binder(),
    ty::PredicateKind::Clause(ty::ClauseKind::Trait(trait_predicate)) if {
        trait_predicate.def_id() == tcx.require_lang_item(LangItem::Sized, None)
    })
  }

  // TODO: I'm not 100% that this is the correct metric.
  fn is_ty_unknown(&self, _tcx: &TyCtxt<'tcx>) -> bool {
    matches!(self.kind().skip_binder(),
    ty::PredicateKind::Clause(ty::ClauseKind::Trait(trait_predicate)) if {
        trait_predicate.self_ty().is_ty_var()
    })
  }

  fn is_trait_predicate(&self) -> bool {
    matches!(
      self.kind().skip_binder(),
      ty::PredicateKind::Clause(ty::ClauseKind::Trait(_trait_predicate))
    )
  }

  fn is_necessary(&self, tcx: &TyCtxt<'tcx>) -> bool {
    // NOTE: predicates of the form `_: TRAIT` and `(): TRAIT` are useless. The first doesn't have
    // any information about the type of the Self var, and I've never understood why the latter
    // occurs so frequently.
    self.is_trait_predicate()
      && !(self.is_unit_impl_trait(tcx)
        || self.is_ty_unknown(tcx)
        || self.is_ty_impl_sized(tcx))
  }
}

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

struct TyVarEraserVisitor<'a, 'tcx: 'a> {
  infcx: &'a InferCtxt<'tcx>,
}

// FIXME: these placeholders are a huge hack, there's definitely
// something better we could do here.
macro_rules! gen_placeholders {
  ($( [$f:ident $n:literal],)*) => {$(
    fn $f(&self) -> Ty<'tcx> {
      Ty::new_placeholder(self.infcx.tcx, ty::PlaceholderType {
        universe: self.infcx.universe(),
        bound: ty::BoundTy {
          var: ty::BoundVar::from_u32(ty::BoundVar::MAX_AS_U32 - $n),
          kind: ty::BoundTyKind::Anon,
        },
      })
    })*
  }
}

impl<'a, 'tcx: 'a> TyVarEraserVisitor<'a, 'tcx> {
  gen_placeholders! {
    [ty_placeholder    0],
    [int_placeholder   1],
    [float_placeholder 2],
  }
}

impl<'tcx> TypeFolder<TyCtxt<'tcx>> for TyVarEraserVisitor<'_, 'tcx> {
  fn interner(&self) -> TyCtxt<'tcx> {
    self.infcx.tcx
  }

  fn fold_ty(&mut self, ty: Ty<'tcx>) -> Ty<'tcx> {
    // HACK: I'm not sure if replacing type variables with
    // an anonymous placeholder is the best idea. It is *an*
    // idea, certainly. But this should only happen before hashing.
    match ty.kind() {
      ty::Infer(ty::TyVar(_)) => self.ty_placeholder(),
      ty::Infer(ty::IntVar(_)) => self.int_placeholder(),
      ty::Infer(ty::FloatVar(_)) => self.float_placeholder(),
      _ => ty.super_fold_with(self),
    }
  }

  fn fold_binder<T>(&mut self, t: ty::Binder<'tcx, T>) -> ty::Binder<'tcx, T>
  where
    T: TypeFoldable<TyCtxt<'tcx>>,
  {
    let u = self.infcx.tcx.anonymize_bound_vars(t);
    u.super_fold_with(self)
  }
}
