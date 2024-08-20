import type {
  ClauseBound,
  ClauseWithBounds,
  GroupedClauses,
  ImplHeader,
  Ty
} from "@argus/common/bindings";
import { anyElems, isUnitTy } from "@argus/common/func";
import _ from "lodash";
import React, { type PropsWithChildren, useContext } from "react";

import { Toggle } from "../Toggle";
import { AllowProjectionSubst, LocationActionable, TyCtxt } from "../context";
import { PrintDefinitionPath } from "./path";
import { PrintClause } from "./predicate";
import { Angled, CommaSeparated, Kw, PlusSeparated, nbsp } from "./syntax";
import {
  PrintBinder,
  PrintGenericArg,
  PrintPolarity,
  PrintRegion,
  PrintTy,
  PrintTyKind
} from "./ty";

// NOTE: it looks ugly, but we need to disable projection substitution for all parts
// of the impl blocks.
export const PrintImplHeader = ({ o }: { o: ImplHeader }) => {
  console.debug("Printing ImplHeader", o);
  const genArgs = _.map(o.args, arg => (
    <AllowProjectionSubst.Provider value={false}>
      <PrintGenericArg o={arg} />
    </AllowProjectionSubst.Provider>
  ));
  const argsWAngle =
    genArgs.length === 0 ? null : (
      <AllowProjectionSubst.Provider value={false}>
        <Angled>
          <Toggle
            summary={".."}
            Children={() => <CommaSeparated components={genArgs} />}
          />
        </Angled>
      </AllowProjectionSubst.Provider>
    );

  const location = o.l;
  const LocationAction = useContext(LocationActionable);
  const LocationWrapper =
    location === undefined
      ? React.Fragment
      : ({ children }: PropsWithChildren) => (
          <LocationAction location={location}>{children}</LocationAction>
        );

  return (
    <AllowProjectionSubst.Provider value={false}>
      <LocationWrapper>
        <Kw>impl</Kw>
      </LocationWrapper>
      {argsWAngle} <PrintDefinitionPath o={o.name} /> <Kw>for</Kw>
      {nbsp}
      <PrintTy o={o.selfTy} />
      <PrintWhereClause
        predicates={o.predicates}
        tysWOBound={o.tysWithoutDefaultBounds}
      />
    </AllowProjectionSubst.Provider>
  );
};

export const PrintGroupedClauses = ({ o }: { o: GroupedClauses }) => {
  console.debug("Printing GroupedClauses", o);
  const Inner = ({ value }: { value: ClauseWithBounds }) => (
    <PrintClauseWithBounds o={value} />
  );
  const groupedClauses = _.map(o.grouped, (group, idx) => (
    <div className="WhereConstraint" key={idx}>
      <PrintBinder binder={group} Child={Inner} />
    </div>
  ));
  const noGroupedClauses = _.map(o.other, (clause, idx) => (
    <div className="WhereConstraint" key={idx}>
      <PrintClause o={clause} />
    </div>
  ));
  return (
    <>
      {groupedClauses}
      {noGroupedClauses}
    </>
  );
};

export const PrintWhereClause = ({
  predicates,
  tysWOBound
}: {
  predicates: GroupedClauses;
  tysWOBound: Ty[];
}) => {
  if (!anyElems(predicates.grouped, predicates.other, tysWOBound)) {
    return null;
  }

  const whereHoverContent = () => (
    <div className="DirNode WhereConstraintArea">
      <PrintGroupedClauses o={predicates} />
      {_.map(tysWOBound, (ty, idx) => (
        <div className="WhereConstraint" key={idx}>
          <PrintTy o={ty} />: ?Sized
        </div>
      ))}
    </div>
  );

  return (
    <>
      {" "}
      <Kw>where</Kw>
      {nbsp}
      <Toggle summary={".."} Children={whereHoverContent} />
    </>
  );
};

const PrintClauseWithBounds = ({ o }: { o: ClauseWithBounds }) => {
  // Sort the bounds to be Ty: Fn() + Trait + Region
  const sortedBounds = _.sortBy(o.bounds, bound =>
    "FnTrait" in bound
      ? 0
      : "Trait" in bound
        ? 1
        : "Region" in bound
          ? 2
          : undefined
  );

  const boundComponents = _.map(sortedBounds, bound => (
    <PrintClauseBound o={bound} />
  ));

  return (
    <>
      <PrintTy o={o.ty} />: <PlusSeparated components={boundComponents} />
    </>
  );
};

const PrintClauseBound = ({ o }: { o: ClauseBound }) => {
  const tyCtxt = useContext(TyCtxt)!;
  if ("FnTrait" in o) {
    const [polarity, path, res] = o.FnTrait;
    const tyKind = tyCtxt.interner[res];
    const arrow = isUnitTy(tyKind) ? null : (
      <>
        {nbsp}
        {"->"}
        {nbsp}
        <PrintTyKind o={tyKind} />
      </>
    );
    return (
      <>
        <PrintPolarity o={polarity} />
        <PrintDefinitionPath o={path} />
        {arrow}
      </>
    );
  } else if ("Trait" in o) {
    const [polarity, path] = o.Trait;
    return (
      <>
        <PrintPolarity o={polarity} />
        <PrintDefinitionPath o={path} />
      </>
    );
  } else if ("Region" in o) {
    return <PrintRegion o={o.Region} />;
  }

  throw new Error("Unknown clause bound", o);
};
