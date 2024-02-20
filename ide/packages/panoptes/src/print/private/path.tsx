import { DefinedPath, PathSegment } from "@argus/common/bindings";
import _ from "lodash";
import React from "react";

import { HoverInfo } from "../../HoverInfo";
import { takeRightUntil } from "../../utilities/func";
import { Angled, CommaSeparated, Kw } from "./syntax";
import { PrintGenericArg, PrintTy } from "./ty";

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

  const hoverContent = () => (
    <div className="DirNode">
      <span className="DefPathFull">
        <PrintDefPathFull o={o} />
      </span>
    </div>
  );

  const [prefix, rest] = pruneToShortPath(o);

  return (
    <div className="DefPathContainer">
      <HoverInfo Content={hoverContent}>
        <span className="DefPathShort">
          <PrintSegments o={prefix} />
        </span>
      </HoverInfo>
      <PrintSegments o={rest} />
    </div>
  );
};

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

export const PrintDefPathShort = ({ o }: { o: DefinedPath }) => {};

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
        o.disambiguator !== undefined && o.disambiguator != 0
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
    case "GenericDelimiters": {
      // We don't want empty <> on the end of types
      if (o.inner.length === 0) {
        return null;
      }
      return (
        <Angled>
          <PrintDefPathFull o={o.inner} />
        </Angled>
      );
    }
    case "CommaSeparated": {
      const Mapper =
        o.kind.type === "GenericArg"
          ? PrintGenericArg
          : ({ o }: { o: any }) => {
              throw new Error("Unknown comma separated kind", o);
            };

      const components = _.map(o.entries, entry => () => <Mapper o={entry} />);

      return <CommaSeparated components={components} />;
    }
    default:
      throw new Error("Unknown path segment", o);
  }
};

// <impl PATH for TY>
export const PrintImplFor = ({ path, ty }: { path?: DefinedPath; ty: any }) => {
  const p =
    path === undefined ? null : (
      <>
        <PrintDefPathFull o={path} />
        <Kw>for</Kw>{" "}
      </>
    );

  return (
    <>
      <Kw>impl</Kw> {p}
      <PrintTy o={ty} />
    </>
  );
};

// <TY as PATH>
export const PrintImplAs = ({ path, ty }: { path?: DefinedPath; ty: any }) => {
  const p =
    path === undefined ? null : (
      <>
        {" "}
        <Kw>as</Kw> <PrintDefPathFull o={path} />
      </>
    );

  return (
    <>
      <PrintTy o={ty} /> {p}
    </>
  );
};
