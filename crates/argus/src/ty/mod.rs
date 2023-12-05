//! Wrapper types for rustc data that don't implement `serde::Serialize`.

use rustc_middle::ty::{PredicateKind, TyCtxt, TraitPredicate, TraitRef};

use serde::Serialize;

#[derive(Serialize)]
#[serde(remote = "PredicateKind")]
enum PredicateKindDef<'tcx> {
    Clause(ClauseKind<TyCtxt<'tcx>>),
    ObjectSafe(DefId),
    Subtype(SubtypePredicate<'tcx>),
    Coerce(CoercePredicate<'tcx>),
    ConstEquate(Const<'tcx>, Const<'tcx>),
    Ambiguous,
    AliasRelate(Term<'tcx>, Term<'tcx>, AliasRelationDirection),
}

#[derive(Serialize)]
#[serde(remote = "ClauseKind")]
pub enum ClauseKindDef {
    Trait(TyCtxt::TraitPredicate),
    RegionOutlives(TyCtxt::RegionOutlivesPredicate),
    TypeOutlives(TyCtxt::TypeOutlivesPredicate),
    Projection(TyCtxt::ProjectionPredicate),
    ConstArgHasType(TyCtxt::Const, TyCtxt::Ty),
    WellFormed(TyCtxt::GenericArg),
    ConstEvaluatable(TyCtxt::Const),
}

#[derive(Serialize)]
#[serde(remote = "TraitPredicate")]
pub struct TraitiPredicateDef<'tcx> {
    pub trait_ref: TraitRef<'tcx>,
    pub polarity: ImplPolarity,
}

#[derive(Serialize)]
#[serde(remote = "TraitRef")]
pub struct TraitRefDef<'tcx> {
    pub def_id: DefId,
    pub args: GenericArgsRef<'tcx>,
    #[serde(skip)]
    pub _use_trait_ref_new_instead: (),
}

#[derive(Serialize)]
#[serde(remote = "ImplPolarity")]
pub enum ImplPolarityDef {
    Positive,
    Negative,
    Reservation,
}

#[derive(Serialize)]
#[serde(remote = "OutlivesPredicate")]
pub struct OutlivesPredicateDef<A: Serialize, B: Serialize>(pub A, pub B);

// RegionOutlivesPredicate
// OutlivesPredicate<Region,  Region>

// TypeOutlivesPredicate
// OutlivesPredicate<Ty,  Region>

// ProjectionPredicate
#[derive(Serialize)]
#[serde(remote = "ProjectionPredicate")]
pub struct ProjectionPredicateDef<'tcx> {
    pub projection_ty: AliasTy<'tcx>,
    #[serde(skip)]
    term: Term<'tcx>,
}

#[derive(Serialize)]
#[serde(remote = "AliasTy")]
pub struct AliasTyDef<'tcx> {
    pub args: GenericArgsRef<'tcx>,
    pub def_id: DefId,
    #[serde(skip)]
    _use_alias_ty_new_instead: (),
}

// Const 
// Ty 
// GenericArg