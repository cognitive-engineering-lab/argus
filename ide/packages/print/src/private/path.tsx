import type { DefinedPath, PathSegment } from "@argus/common/bindings";
import { isNamedGenericArg, takeRightUntil } from "@argus/common/func";
import _ from "lodash";
import React, { useContext } from "react";
import { Toggle } from "../Toggle";
import { AllowPathTrim, AllowToggle, DefPathRender, TyCtxt } from "../context";
import { Angled, CommaSeparated, Kw, nbsp } from "./syntax";
import { PrintGenericArg, PrintTy } from "./ty";

// Special case the printing for associated types. Things that look like
// `<Type as Trait>::AssocType`, we want to print this as `<...>::AssocType` so that
// people can visually see that this is an associated type.
function isAssociatedType(
  o: DefinedPath
): o is [PathSegment & { type: "GenericDelimiters" }, ...DefinedPath] {
  return o.length > 1 && o[0].type === "GenericDelimiters";
}

function pruneToShortPath(o: DefinedPath): [DefinedPath, DefinedPath] {
  // Take the rightmost segments that form a full "path".
  const prefix = takeRightUntil(
    o,
    segment =>
      segment.type === "Ty" ||
      segment.type === "DefPathDataName" ||
      segment.type === "Impl"
  );

  // Take the leftmost segments that are named, these will have a hover
  // element attached to them.
  return [[prefix[0]], _.slice(prefix, 1)];
}

export const PrintValuePath = ({ o }: { o: DefinedPath }) => {
  return <PrintDefPath o={o} />;
};

// NOTE: when we aren't hovering over the path, we just
// want to show the last segment. On hover, it should be the fully
// qualified path. (At least that's the current idea.)
export const PrintDefPath = ({ o }: { o: DefinedPath }) => {
  if (o.length === 0) {
    return null;
  }

  const tyCtxt = useContext(TyCtxt)!;
  const toggle = useContext(AllowPathTrim);
  if (!toggle) {
    return <PrintDefPathFull o={o} />;
  }

  const PrintCustomDefPath = useContext(DefPathRender);

  const PrintAsGenericPath = ({
    Prefix,
    Rest
  }: {
    Prefix: React.FC;
    Rest: React.FC;
  }) => {
    return (
      <PrintCustomDefPath
        fullPath={o}
        ctx={tyCtxt}
        Head={<Prefix />}
        Rest={<Rest />}
      />
    );
  };

  const PrintAsAssociatedType = ({
    o
  }: {
    o: [PathSegment & { type: "GenericDelimiters" }, ...DefinedPath];
  }) => {
    return (
      <PrintAsGenericPath
        Prefix={() => (
          <Angled>
            <Toggle
              summary=".."
              Children={() => <PrintPathSegment o={o[0]} />}
            />
          </Angled>
        )}
        Rest={() => <PrintSegments o={_.slice(o, 1)} />}
      />
    );
  };

  return isAssociatedType(o) ? (
    <PrintAsAssociatedType o={o} />
  ) : (
    (() => {
      const [prefix, rest] = pruneToShortPath(o);
      return (
        <PrintAsGenericPath
          Prefix={() => <PrintSegments o={prefix} />}
          Rest={() => <PrintSegments o={rest} />}
        />
      );
    })()
  );
};

export const PrintDefPathFull = ({ o }: { o: DefinedPath }) => {
  return <PrintSegments o={o} />;
};

const PrintSegments = ({ o }: { o: PathSegment[] }) => {
  return _.map(o, (segment, i) => <PrintPathSegment o={segment} key={i} />);
};

export const PrintPathSegment = ({ o }: { o: PathSegment }) => {
  switch (o.type) {
    case "Colons": {
      return "::";
    }
    case "LocalCrate": {
      return "crate";
    }
    case "RawGuess": {
      return "r#";
    }
    case "Ty": {
      return <PrintTy o={o.ty} />;
    }
    case "DefPathDataName": {
      const suffix =
        o.disambiguator !== undefined && o.disambiguator !== 0
          ? `#${o.disambiguator}`
          : null;
      return (
        <>
          {o.name}
          {suffix}
        </>
      );
    }
    case "Impl": {
      switch (o.kind.type) {
        case "For":
          return <PrintImplFor path={o.path} ty={o.ty} />;
        case "As":
          return <PrintImplAs path={o.path} ty={o.ty} />;
        default:
          throw new Error("Unknown impl kind", o.kind);
      }
    }
    case "AnonImpl": {
      // TODO: we should actually print something here (or send the file snippet).
      return <span>impl@{o.range.toString()}</span>;
    }
    // General case of wrapping segments in angle brackets.
    case "GenericDelimiters": {
      // We don't want empty <> on the end of types
      if (o.inner.length === 0) {
        return null;
      }

      return (
        <PrintInToggleableEnvironment
          bypassToggle={o.inner.length > 3}
          Elem={() => <PrintDefPath o={o.inner} />}
        />
      );
    }
    // Angle brackets used *specifically* for a list of generic arguments.
    case "GenericArgumentList": {
      const namedArgs = _.filter(o.entries, isNamedGenericArg);
      if (namedArgs.length === 0) {
        return null;
      }

      const components = _.map(namedArgs, (arg, i) => (
        <PrintGenericArg o={arg} key={i} />
      ));
      return (
        <PrintInToggleableEnvironment
          bypassToggle={namedArgs.length > 3}
          Elem={() => <CommaSeparated components={components} />}
        />
      );
    }
    default:
      throw new Error("Unknown path segment", o);
  }
};

// NOTE: used as a helper for the `GenericDelimiters` and `GenericArgumentList` segments.
const PrintInToggleableEnvironment = ({
  bypassToggle,
  Elem
}: { bypassToggle: boolean; Elem: React.FC }) => {
  // Use a metric of "type size" rather than inner lenght.
  const useToggle = useContext(AllowToggle) && bypassToggle;
  return (
    // TODO: do we want to allow nested toggles?
    <Angled>
      {useToggle ? <Toggle summary=".." Children={() => <Elem />} /> : <Elem />}
    </Angled>
  );
};

// <impl PATH for TY>
export const PrintImplFor = ({ path, ty }: { path?: DefinedPath; ty: any }) => {
  const p =
    path === undefined ? null : (
      <>
        <PrintDefPath o={path} /> <Kw>for</Kw>
        {nbsp}
      </>
    );

  return (
    <>
      <Kw>impl</Kw>
      {nbsp}
      {p}
      <PrintTy o={ty} />
    </>
  );
};

// <TY as PATH>
export const PrintImplAs = ({ path, ty }: { path?: DefinedPath; ty: any }) => {
  const p =
    path === undefined ? null : (
      <>
        {nbsp}
        <Kw>as</Kw> <PrintDefPath o={path} />
      </>
    );

  return (
    <>
      <PrintTy o={ty} />
      {p}
    </>
  );
};
