import type {
  ClauseBound,
  ClauseWithBounds,
  GroupedClauses,
  ImplHeader,
  PolyClauseKind,
  PolyClauseWithBounds,
  Ty
} from "@argus/common/bindings";
import { anyElems, isUnitTy } from "@argus/common/func";
import _ from "lodash";
import React, {
  type PropsWithChildren,
  type ReactElement,
  useContext
} from "react";

import classNames from "classnames";
import { Toggle } from "../Toggle";
import { AllowProjectionSubst, LocationActionable, TyCtxt } from "../context";
import { Angled, CommaSeparated, Kw, PlusSeparated, nbsp } from "../syntax";
import { PrintDefinitionPath } from "./path";
import { PrintClause } from "./predicate";
import {
  PrintBinder,
  PrintGenericArg,
  PrintPolarity,
  PrintRegion,
  PrintTy,
  PrintTyKind
} from "./ty";

import "./argus.css";

export const WhereConstraintArea = ({
  className,
  children
}: React.PropsWithChildren<{ className?: string }>) => (
  <div className={classNames(className, "WhereConstraintArea")}>{children}</div>
);

export const WhereConstraint = ({ children }: React.PropsWithChildren) => (
  <div className="WhereConstraint">{children}</div>
);

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

export const PrintClauses = ({
  grouped,
  ungrouped,
  tysWOBound
}: {
  grouped: PolyClauseWithBounds[];
  ungrouped: PolyClauseKind[];
  tysWOBound: Ty[];
}) => {
  const Group = ({ value }: { value: PolyClauseWithBounds }) => (
    <PrintBinder
      binder={value}
      Child={({ value }) => <PrintClauseWithBounds o={value} />}
    />
  );
  const Ungrouped = ({ value }: { value: PolyClauseKind }) => (
    <PrintClause o={value} />
  );
  const Unsized = ({ value }: { value: Ty }) => (
    <>
      <PrintTy o={value} />: ?Sized
    </>
  );

  const rawElements /*: (for<T> [T, React.FC<{ value: T }>])[] */ = [
    ..._.map(grouped, group => [group, Group] as const),
    ..._.map(ungrouped, ungroup => [ungroup, Ungrouped] as const),
    ..._.map(tysWOBound, ty => [ty, Unsized] as const)
  ] as const;

  const elements = _.map(
    rawElements,
    (
      [value, C] /*: for<T> [T, React.FC<{ value: T }> */,
      idx
    ): ReactElement => (
      <WhereConstraint key={idx}>
        <C value={value as any} />
      </WhereConstraint>
    )
  );

  // TODO: the `{elements}` should be wrapped in a `CommaSeparated` component,
  // but comma placement is done manually in the `WhereConstraintsArea`---for now. See CSS
  // file for more comments.
  return <WhereConstraintArea>{elements}</WhereConstraintArea>;
};

const PrintWhereClause = ({
  predicates: { grouped, other: ungrouped },
  tysWOBound
}: {
  predicates: GroupedClauses;
  tysWOBound: Ty[];
}) => {
  if (!anyElems(grouped, ungrouped, tysWOBound)) {
    return null;
  }

  const content = (
    <PrintClauses
      grouped={grouped}
      ungrouped={ungrouped}
      tysWOBound={tysWOBound}
    />
  );

  return (
    <>
      <br />
      <Kw>where</Kw>
      {nbsp}
      <Toggle summary={".."} Children={() => content} />
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
