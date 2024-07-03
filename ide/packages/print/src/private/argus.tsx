import type {
  ClauseBound,
  ClauseWithBounds,
  GroupedClauses,
  ImplHeader,
  Ty
} from "@argus/common/bindings";
import { anyElems } from "@argus/common/func";
import _ from "lodash";
import React from "react";

import { Toggle } from "../Toggle";
import { AllowProjectionSubst } from "../context";
import { PrintDefPath } from "./path";
import { PrintClause } from "./predicate";
import { Angled, CommaSeparated, Kw, PlusSeparated } from "./syntax";
import { PrintGenericArg, PrintPolarity, PrintRegion, PrintTy } from "./ty";

// NOTE: it looks ugly, but we need to disable projection substitution for all parts
// of the impl blocks.
export const PrintImplHeader = ({ o }: { o: ImplHeader }) => {
  console.debug("Printing ImplHeader", o);
  const genArgs = _.map(o.args, arg => () => (
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

  return (
    <AllowProjectionSubst.Provider value={false}>
      <Kw>impl</Kw>
      {argsWAngle} <PrintDefPath o={o.name} /> <Kw>for</Kw>{" "}
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
  const groupedClauses = _.map(o.grouped, (group, idx) => (
    <div className="WhereConstraint" key={idx}>
      <PrintClauseWithBounds o={group} />
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
      <Kw>where</Kw> <Toggle summary={".."} Children={whereHoverContent} />
    </>
  );
};

const PrintClauseWithBounds = ({ o }: { o: ClauseWithBounds }) => {
  const [traits, lifetimes] = _.partition(o.bounds, bound => "Trait" in bound);
  const traitBounds = _.map(traits, bound => () => (
    <PrintClauseBound o={bound} />
  ));
  const lifetimeBounds = _.map(lifetimes, bound => () => (
    <PrintClauseBound o={bound} />
  ));
  const boundComponents = _.concat(traitBounds, lifetimeBounds);

  return (
    <>
      <PrintTy o={o.ty} />: <PlusSeparated components={boundComponents} />
    </>
  );
};

const PrintClauseBound = ({ o }: { o: ClauseBound }) => {
  if ("Trait" in o) {
    const [polarity, path] = o.Trait;
    return (
      <>
        <PrintPolarity o={polarity} />
        <PrintDefPath o={path} />
      </>
    );
  }
  if ("Region" in o) {
    return <PrintRegion o={o.Region} />;
  }
  throw new Error("Unknown clause bound", o);
};
