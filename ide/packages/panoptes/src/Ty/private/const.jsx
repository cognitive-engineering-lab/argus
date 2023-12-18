import React from "react";

export const PrintConst = ({ o }) => {
  //   Infer(#[serde(with = "InferConstDef")] InferConst),
  //   Bound(#[serde(skip)] DebruijnIndex, #[serde(with = "BoundConstDef")] <TyCtxt<'tcx> as Interner>::BoundConst),
  //   Placeholder(#[serde(with = "PlaceholderConstDef")] <TyCtxt<'tcx> as Interner>::PlaceholderConst),
  //   Unevaluated(#[serde(with = "AliasConstDef")] <TyCtxt<'tcx> as Interner>::AliasConst),
  //   Value(#[serde(with = "ValueConstDef")] <TyCtxt<'tcx> as Interner>::ValueConst),
  //   Error(#[serde(skip)] <TyCtxt<'tcx> as Interner>::ErrorGuaranteed),
  //   Expr(#[serde(with = "ExprConstDef")] <TyCtxt<'tcx> as Interner>::ExprConst),

  if ("Infer" in o) {
    throw new Error("TODO");
  } else if ("Bound" in o) {
    throw new Error("TODO");
  } else if ("Placeholder" in o) {
    throw new Error("TODO");
  } else if ("Unevaluated" in o) {
    throw new Error("TODO");
  } else if ("Value" in o) {
    throw new Error("TODO");
  } else if ("Error" in o) {
    return "{{error}}";
  } else if ("Expr" in o) {
    // FIXME: this is what rustc does, but can't we just print
    // the full expression?
    return "{{const expr}}";
  } else {
    throw new Error("Unknown const", o);
  }
};
