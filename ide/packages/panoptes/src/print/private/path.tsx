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

  return (
    <div className="DefPathContainer">
      <HoverInfo
        Content={() => (
          <div className="DirNode">
            <span className="DefPathFull">
              <PrintDefPathFull o={o} />
            </span>
          </div>
        )}
      >
        <span className="DefPathShort">
          <PrintDefPathShort o={o} />
        </span>
      </HoverInfo>
    </div>
  );
};

// PathSegment[]
export const PrintDefPathShort = ({ o }: { o: DefinedPath }) => {
  console.debug("Printing def path short: ", o);
  const prefix = takeRightUntil(o, segment => {
    return (
      segment.type === "Ty" ||
      segment.type === "DefPathDataName" ||
      segment.type === "Impl"
    );
  });

  return (
    <span>
      {_.map(prefix, (segment, i) => {
        return <PrintPathSegment o={segment} key={i} />;
      })}
    </span>
  );
};

// PathSegment[]
export const PrintDefPathFull = ({ o }: { o: DefinedPath }) => {
  return (
    <>
      {_.map(o, (segment, i) => {
        return <PrintPathSegment o={segment} key={i} />;
      })}
    </>
  );
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
        <span>
          {o.name}
          {suffix}
        </span>
      );
    }
    case "Impl": {
      switch (o.kind.type) {
        case "For":
          return <PrintImplFor path={o.path} ty={o.ty} />;
        case "As":
          return <PrintImplAs path={o.path} ty={o.ty} />;
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
