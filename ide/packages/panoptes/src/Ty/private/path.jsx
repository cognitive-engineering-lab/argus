import _ from "lodash";
import React from "react";

import { PrintGenericArg, PrintTy } from "./ty";
import { intersperse, takeRightUntil } from "./utilities";

// NOTE: when we aren't hovering over the path, we just
// want to show the last segment. On hover, it should be the fully
// qualified path. (At least that's the current idea.)
export const PrintDefPath = ({ o }) => {
  if (o.length === 0) {
    return "";
  }

  return (
    <div className="DefPathContainer">
      <span className="DefPathShort">
        <PrintDefPathShort o={o} />
      </span>
      <span className="DefPathFull">
        <PrintDefPathFull o={o} />
      </span>
    </div>
  );
};

// PathSegment[]
export const PrintDefPathShort = ({ o }) => {
  const prefix = takeRightUntil(o, segment => {
    return (
      segment.type === "crate" ||
      segment.type === "ty" ||
      segment.type === "defPathDataName" ||
      segment.type === "implFor" ||
      segment.type === "implAs"
    );
  });
  console.log("Prefix", prefix);

  return (
    <span>
      {_.map(prefix, (segment, i) => {
        return <PrintPathSegment o={segment} key={i} />;
      })}
    </span>
  );
};

// PathSegment[]
export const PrintDefPathFull = ({ o }) => {
  return (
    <span>
      {_.map(o, (segment, i) => {
        return <PrintPathSegment o={segment} key={i} />;
      })}
    </span>
  );
};

export const PrintPathSegment = ({ o }) => {
  switch (o.type) {
    case "colons": {
      return "::";
    }
    case "localCrate": {
      return "crate";
    }
    case "rawGuess": {
      return "r#";
    }
    case "defPathDataName": {
      const suffix = o.disambiguator != 0 ? `#${o.disambiguator}` : "";
      return (
        <span>
          {o.name}
          {suffix}
        </span>
      );
    }
    case "crate": {
      return o.name;
    }
    case "ty": {
      return <PrintTy o={o.ty} />;
    }
    case "genericDelimiters": {
      // We don't want empty <> on the end of types
      if (o.inner.length === 0) {
        return "";
      }

      console.log("genericDelimiters", o);

      let [lt, gt] = ["<", ">"];
      return (
        <span>
          {lt}
          <PrintDefPathFull o={o.inner} />
          {gt}
        </span>
      );
    }
    case "commaSeparated": {
      const doInner =
        o.kind.type === "genericArg"
          ? (e, i) => {
              return <PrintGenericArg o={e} key={i} />;
            }
          : (_e, _i) => {
              throw new Error("Unknown comma separated kind", o);
            };
      console.log("CommaSeparated", o);
      return <>{intersperse(o.entries, ", ", doInner)}</>;
    }
    case "implFor": {
      const path =
        o.path === undefined ? (
          ""
        ) : (
          <span>
            <PrintDefPathFull o={o.path} />
            for
          </span>
        );
      return (
        <span>
          impl {path} <PrintTy o={o.ty} />
        </span>
      );
    }
    case "implAs": {
      const path =
        o.path === undefined ? (
          ""
        ) : (
          <span>
            as
            <PrintDefPathFull o={o.path} />
          </span>
        );
      return (
        <span>
          <PrintTy o={o.ty} />
          {path}
        </span>
      );
    }
  }
};

export const PrintDefPathData = ({ o }) => {
  // TODO: see how they actually do it in rustc
  if ("CrateRoot" in o) {
    return "crate";
  } else if ("Impl" in o) {
    return "impl";
  } else if ("ForeignMod" in o) {
    return "foreign mod";
  } else if ("Use" in o) {
    return "use";
  } else if ("GlobalAsm" in o) {
    return "asm";
  } else if ("TypeNs" in o) {
    return o.TypeNs;
  } else if ("ValueNs" in o) {
    return o.TypeNs;
  } else if ("MacroNs" in o) {
    return o.MacroNs;
  } else if ("LifetimeNs" in o) {
    return o.LifetimeNs;
  } else if ("Ctor" in o) {
    return "{{constructor}}";
  } else if ("AnonConst" in o) {
    return "{{anon_constructor}}";
  } else if ("ImplTrait" in o) {
    return "impl-trait";
  } else if ("ImplTraitAssocTy" in o) {
    return "impl-trait-assoc-ty";
  } else {
    throw new Error("Unknown def path data", o);
  }
};
