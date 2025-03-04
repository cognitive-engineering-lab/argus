import type {
  AliasRelationDirection,
  BoundConstness,
  Clause,
  ClauseKind,
  GoalPredicate,
  ParamEnv,
  PolyPredicateKind,
  PredicateKind,
  PredicateObligation,
  TraitPredicate
} from "@argus/common/bindings";
import { anyElems } from "@argus/common/func";
import React from "react";

import { HoverInfo } from "../HoverInfo";
import { IcoNote } from "../Icons";
import MonoSpace from "../MonoSpace";
import { PrintClauses } from "./argus";
import { PrintConst } from "./const";
import { PrintDefinitionPath } from "./path";
import { PrintTerm } from "./term";
import {
  PrintAliasTerm,
  PrintBinder,
  PrintGenericArg,
  PrintPolarity,
  PrintRegion,
  PrintTy
} from "./ty";

export const PrintPredicateObligation = ({ o }: { o: PredicateObligation }) => {
  const hoverContent = !anyElems(
    o.paramEnv.grouped,
    o.paramEnv.other
  ) ? null : (
    <HoverInfo
      Content={() => (
        <>
          <p>Facts in the type environment</p>
          <MonoSpace>
            <PrintParamEnv o={o.paramEnv} />
          </MonoSpace>
        </>
      )}
    >
      {" "}
      <IcoNote />
    </HoverInfo>
  );

  return (
    <>
      <PrintBinderPredicateKind o={o.predicate} />
      {hoverContent}
    </>
  );
};

export const PrintGoalPredicate = ({ o }: { o: GoalPredicate }) => (
  // NOTE: goals and obligations aren't the same thing, but they
  // currently have the same semantic structure.
  <PrintPredicateObligation o={o} />
);

export const PrintParamEnv = ({ o }: { o: ParamEnv }) => (
  <div className="WhereConstraintArea">
    <PrintClauses grouped={o.grouped} ungrouped={o.other} tysWOBound={[]} />
  </div>
);

export const PrintBinderPredicateKind = ({ o }: { o: PolyPredicateKind }) => {
  const Inner = ({ value }: { value: PredicateKind }) => (
    <PrintPredicateKind o={value} />
  );
  return <PrintBinder binder={o} Child={Inner} />;
};

export const PrintPredicateKind = ({ o }: { o: PredicateKind }) => {
  if (o === "Ambiguous") {
    return "ambiguous";
  } else if ("Clause" in o) {
    return <PrintClauseKind o={o.Clause} />;
  } else if ("DynCompatible" in o) {
    return (
      <>
        The trait <PrintDefinitionPath o={o.DynCompatible} /> is object-safe
      </>
    );
  } else if ("Subtype" in o) {
    const subty = o.Subtype;
    const st = "<:";
    return (
      <>
        <PrintTy o={subty.a} /> {st} <PrintTy o={subty.b} />
      </>
    );
  } else if ("Coerce" in o) {
    const coerce = o.Coerce;
    return (
      <>
        <PrintTy o={coerce.a} /> → <PrintTy o={coerce.b} />
      </>
    );
  } else if ("ConstEquate" in o) {
    const [a, b] = o.ConstEquate;
    return (
      <>
        <PrintConst o={a} /> = <PrintConst o={b} />
      </>
    );
  } else if ("AliasRelate" in o) {
    const [a, b, dir] = o.AliasRelate;
    return (
      <>
        <PrintTerm o={a} /> <PrintAliasRelationDirection o={dir} />{" "}
        <PrintTerm o={b} />
      </>
    );
  } else if ("NormalizesTo" in o) {
    return (
      <>
        <PrintAliasTerm o={o.NormalizesTo.alias} /> normalizes to{" "}
        <PrintTerm o={o.NormalizesTo.term} />
      </>
    );
  } else {
    throw new Error("Unknown predicate kind", o);
  }
};

export const PrintAliasRelationDirection = ({
  o
}: {
  o: AliasRelationDirection;
}) => {
  if (o === "Equate") {
    return "==";
  }
  if (o === "Subtype") {
    return "<:";
  }
  throw new Error("Unknown alias relation direction", o);
};

export const PrintClause = ({ o }: { o: Clause }) => {
  const Inner = ({ value }: { value: ClauseKind }) => (
    <PrintClauseKind o={value} />
  );
  return <PrintBinder binder={o} Child={Inner} />;
};

export const PrintClauseKind = ({ o }: { o: ClauseKind }) => {
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
      <>
        <PrintTy o={to.a} />: <PrintRegion o={to.b} />
      </>
    );
  } else if ("Projection" in o) {
    const proj = o.Projection;
    return (
      <span>
        <PrintAliasTerm o={proj.projection_term} /> =={" "}
        <PrintTerm o={proj.term} />
      </span>
    );
  } else if ("ConstArgHasType" in o) {
    const [c, ty] = o.ConstArgHasType;
    return (
      <>
        const <PrintConst o={c} /> as type <PrintTy o={ty} />
      </>
    );
  } else if ("WellFormed" in o) {
    return (
      <>
        <PrintGenericArg o={o.WellFormed} /> well-formed
      </>
    );
  } else if ("ConstEvaluatable" in o) {
    return (
      <>
        <PrintConst o={o.ConstEvaluatable} /> can be evaluated
      </>
    );
  } else if ("HostEffect" in o) {
    <PrintTraitPredicate
      o={o.HostEffect.predicate}
      constness={o.HostEffect.constness}
    />;
  } else {
    throw new Error("Unknown clause kind", o);
  }
};

export const PrintBoundConstness = ({ o }: { o: BoundConstness }) => {
  if (o === "C") {
    return "const ";
  }
  return null;
};

export const PrintTraitPredicate = ({
  o,
  constness
}: { o: TraitPredicate; constness?: BoundConstness }) => {
  return (
    <>
      <PrintTy o={o.self_ty} />:{" "}
      {constness ? (
        <>
          <PrintBoundConstness o={constness} />{" "}
        </>
      ) : null}
      <PrintPolarity o={o.polarity} />
      <PrintDefinitionPath o={o.trait_ref} />
    </>
  );
};
