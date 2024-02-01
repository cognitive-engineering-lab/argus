import React from "react";

import { PrintConst } from "./const";
import { PrintDefPath } from "./path";
import { PrintTerm } from "./term";
import { PrintBinder, PrintGenericArg, PrintTy } from "./ty";

export const PrintGoalPredicate = ({ o }) => {
  // NOTE: by default just print the predicate, not the env.
  return <PrintBinderPredicateKind o={o.predicate} />;
};

export const PrintBinderPredicateKind = ({ o }) => {
  const inner = o => <PrintPredicateKind o={o} />;
  return <PrintBinder binder={o} innerF={inner} />;
};

export const PrintPredicateKind = ({ o }) => {
  if ("Clause" in o) {
    return <PrintClauseKind o={o.Clause} />;
  } else if ("ObjectSafe" in o) {
    return (
      <span>
        The trait <PrintDefPath o={o.ObjectSafe} /> is object-safe
      </span>
    );
  } else if ("Subtype" in o) {
    const subty = o.Subtype;
    const st = "<:";
    return (
      <span>
        <PrintTy o={subty.a} /> {st} <PrintTy o={subty.b} />
      </span>
    );
  } else if ("Coerce" in o) {
    const coerce = o.Coerce;
    return (
      <span>
        <PrintTy o={coerce.a} /> → <PrintTy o={coerce.b} />
      </span>
    );
  } else if ("ConstEquate" in o) {
    const [a, b] = o.ConstEquate;
    return (
      <span>
        <PrintConst o={a} /> = <PrintConst o={b} />
      </span>
    );
  } else if ("Ambiguous" in o) {
    return "ambiguous";
  } else if ("AliasRelate" in o) {
    const [a, b, dir] = o.AliasRelate;
    return (
      <span>
        <PrintTerm o={a} /> <PrintAliasRelationDirection o={dir} />{" "}
        <PrintTerm o={b} />
      </span>
    );
  } else {
    throw new Error("Unknown predicate kind", o);
  }
};

export const PrintAliasRelationDirection = ({ o }) => {
  if (o === "Equate") {
    return "==";
  } else if (o === "Subtype") {
    return "<:";
  } else {
    throw new Error("Unknown alias relation direction", o);
  }
};

export const PrintClauseKind = ({ o }) => {
  if ("Trait" in o) {
    return <PrintTraitPredicate o={o.Trait} />;
  } else if ("RegionOutlives" in o) {
    const ro = o.RegionOutlives;
    return (
      <span>
        <PrintRegion o={ro.a} />: <PrintRegion o={ro.b} />
      </span>
    );
  } else if ("TypeOutlives" in o) {
    const to = o.TypeOutlives;
    return (
      <span>
        <PrintTy o={to.a} />: <PrintRegion o={to.b} />
      </span>
    );
  } else if ("Projection" in o) {
    const proj = o.Projection;
    return (
      <span>
        <PrintAliasTy o={proj.projection_ty} /> == <PrintTerm o={proj.term} />
      </span>
    );
  } else if ("ConstArgHasType" in o) {
    const [c, ty] = o.ConstArgHasType;
    return (
      <span>
        const <PrintConst o={c} /> as type <PrintTy o={ty} />
      </span>
    );
  } else if ("WellFormed" in o) {
    return (
      <span>
        <PrintGenericArg o={o.WellFormed} /> well-formed
      </span>
    );
  } else if ("ConstEvaluatable" in o) {
    return (
      <span>
        <PrintConst o={o.ConstEvaluatable} /> can be evaluated
      </span>
    );
  } else {
    throw new Error("Unknown clause kind", o);
  }
};

export const PrintTraitPredicate = ({ o }) => {
  let polarity = o.polarity === "Negative" ? "!" : "";
  return (
    <>
      <span>{polarity}</span>
      <PrintTraitRef o={o.trait_ref} />
    </>
  );
};

export const PrintTraitRef = ({ o }) => {
  return (
    <span>
      <PrintTy o={o.self_ty} />: <PrintDefPath o={o.trait_path} />
    </span>
  );
};
