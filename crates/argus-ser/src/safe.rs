use rustc_hir::def_id::DefId;
use rustc_middle::ty;
use rustc_trait_selection::traits::solve;
use serde::Serialize;
#[cfg(feature = "testing")]
use ts_rs::TS;

use crate::{path, ty as myty};

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
#[cfg_attr(feature = "testing", ts(rename = "GoalPredicateDefSafeWrapper"))]
pub struct GoalPredicateDef<'tcx>(
  #[serde(with = "myty::Goal__PredicateDef")]
  #[cfg_attr(feature = "testing", ts(type = "GoalPredicate"))]
  pub solve::Goal<'tcx, ty::Predicate<'tcx>>,
);

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
#[cfg_attr(feature = "testing", ts(rename = "PathDefNoArgsSafeWrapper"))]
pub struct PathDefNoArgs(
  #[serde(with = "path::PathDefNoArgs")]
  #[cfg_attr(feature = "testing", ts(type = "PathDefNoArgs"))]
  pub DefId,
);

#[derive(Serialize)]
#[cfg_attr(feature = "testing", derive(TS))]
#[cfg_attr(feature = "testing", ts(export))]
#[cfg_attr(
  feature = "testing",
  ts(rename = "TraitRefPrintOnlyTraitPathDefSafeWrapper")
)]
pub struct TraitRefPrintOnlyTraitPathDef<'tcx>(
  #[serde(with = "myty::TraitRefPrintOnlyTraitPathDef")]
  #[cfg_attr(feature = "testing", ts(type = "TraitRefPrintOnlyTraitPath"))]
  pub ty::TraitRef<'tcx>,
);
