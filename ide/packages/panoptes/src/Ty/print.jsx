import _ from "lodash";
import React, { useState } from "react";

import "./print.css";

function intersperse(arr, sep, proc = undefined) {
  const doInner = proc === undefined ? (e, _i) => e : proc;
  return _.flatMap(arr, (entry, i) => {
    let e = doInner(entry, i);
    return arr.length - 1 === i ? [e] : [e, sep];
  });
}

export const PrettyObligation = ({ obligation }) => {
  console.log("Printing Obligation", obligation);
  return <PrintBinderPredicateKind o={obligation.data} />;
};

const PrintBinder = ({ binder, innerF }) => {
  return innerF(binder.value);
};

const PrintBinderPredicateKind = ({ o }) => {
  const inner = o => <PrintPredicateKind o={o} />;
  return <PrintBinder binder={o} innerF={inner} />;
};

const PrintPredicateKind = ({ o }) => {
  if ("Clause" in o) {
    return <PrintClauseKind o={o.Clause} />;
  } else if ("ObjectSafe" in o) {
    throw new Error("TODO");
  } else if ("Subtype" in o) {
    throw new Error("TODO");
  } else if ("Coerce" in o) {
    throw new Error("TODO");
  } else if ("ConstEquate" in o) {
    throw new Error("TODO");
  } else if ("Ambiguous" in o) {
    throw new Error("TODO");
  } else if ("AliasRelate" in o) {
    throw new Error("TODO");
  } else if ("ClosureKind" in o) {
    throw new Error("TODO");
  } else {
    throw new Error("Unknown predicate kind", o);
  }
};

const PrintClauseKind = ({ o }) => {
  if ("Trait" in o) {
    return <PrintTraitPredicate o={o.Trait} />;
  } else if ("RegionOutlives" in o) {
    throw new Error("TODO");
  } else if ("TypeOutlives" in o) {
    throw new Error("TODO");
  } else if ("Projection" in o) {
    throw new Error("TODO");
  } else if ("ConstArgHasType" in o) {
    throw new Error("TODO");
  } else if ("WellFormed" in o) {
    throw new Error("TODO");
  } else if ("ConstEvaluatable" in o) {
    throw new Error("TODO");
  } else {
    throw new Error("Unknown clause kind", o);
  }
};

const PrintTraitPredicate = ({ o }) => {
  let polarity = o.polarity === "Negative" ? "!" : "";
  return (
    <>
      <span>{polarity}</span>
      <PrintTraitRef o={o.trait_ref} />
    </>
  );
};

const PrintTraitRef = ({ o }) => {
  return (
    <span>
      <PrintTy o={o.self_ty} />: <PrintDefPath o={o.trait_path} />
    </span>
  );
};

// NOTE: when we aren't hovering over the path, we just
// want to show the last segment. On hover, it should be the fully
// qualified path. (At least that's the current idea.)
const PrintDefPath = ({ o }) => {
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

// NOTE: difference between this and _.takeRightWhile is that
// this will include the first element that matches the predicate.
function takeRightUntil(arr, pred) {
  let i = arr.length - 1;
  while (0 <= i) {
    if (pred(arr[i])) {
      break;
    }
    i--;
  }
  return arr.slice(i, arr.length);
}

// PathSegment[]
const PrintDefPathShort = ({ o }) => {
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
const PrintDefPathFull = ({ o }) => {
  return (
    <span>
      {_.map(o, (segment, i) => {
        return <PrintPathSegment o={segment} key={i} />;
      })}
    </span>
  );
};

const PrintPathSegment = ({ o }) => {
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
        </span>
      );
    }
  }
};

const PrintGenericArg = ({ o }) => {
  console.log("GenericArg", o);

  if ("Type" in o) {
    return <PrintTy o={o.Type} />;
  } else if ("Lifetime" in o) {
    throw new Error("TODO");
  } else if ("Const" in o) {
    throw new Error("TODO");
  } else {
    throw new Error("Unknown generic arg", o);
  }
};

const PrintDefPathData = ({ o }) => {
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

const PrintTy = ({ o }) => {
  console.log("Printing Ty", o);
  return <PrintTyKind o={o} />;
};

// TODO: enums that don't have an inner object need to use a
// comparison, instead of the `in` operator.
const PrintTyKind = ({ o }) => {
  if (o === "Error") {
    return "{error}";
  }

  if ("Bool" in o) {
    return "bool";
  } else if ("Char" in o) {
    return "char";
  } else if ("Int" in o) {
    return <PrintIntTy o={o.Int} />;
  } else if ("Uint" in o) {
    return <PrintUintTy o={o.Uint} />;
  } else if ("Float" in o) {
    return <PrintFloatTy o={o.Float} />;
  } else if ("Adt" in o) {
    return <PrintDefPath o={o.Adt} />;
  } else if ("Str" in o) {
    return "str";
  } else if ("Array" in o) {
    // FIXME: the PrintTy and PrintConst are wrong,
    // we need to pass the right arguments to them.
    return (
      <span>
        [<PrintTy o={o} />; <PrintConst o={o} />]
      </span>
    );
  } else if ("Slice" in o) {
    return (
      <span>
        [<PrintTy o={o.Slice} />]
      </span>
    );
  } else if ("RawPtr" in o) {
    throw new Error("TODO");
  } else if ("Ref" in o) {
    throw new Error("TODO");
  } else if ("FnDef" in o) {
    // FIXME: function definitions should also have a signature
    return <PrintDefPath o={o.FnDef} />;
  } else if ("Never" in o) {
    return "!";
  } else if ("Tuple" in o) {
    return (
      <span>
        (
        {intersperse(o.Tuple, ", ", (e, i) => {
          return <PrintTy o={e} key={i} />;
        })}
        )
      </span>
    );
  } else if ("Placeholder" in o) {
    throw new Error("TODO");
  } else if ("Infer" in o) {
    return <PrintInferTy o={o.Infer} />;
  } else if ("Error" in o) {
    throw new Error("TODO");
  } else {
    throw new Error("Unknown ty kind", o);
  }
};

const PrintInferTy = ({ o }) => {
  if ("ty_var" in o) {
    return <PrintTy o={o.ty_var} />;
  } else if ("infer_var" in o) {
    // NOTE: currently infer_var is just a string.
    return o.infer_var;
  } else {
    throw new Error("Unknown infer ty", o);
  }
};

const PrintTyVar = ({ o }) => {
  if (typeof o === "string" || o instanceof String) {
    return o;
  } else {
    return <PrintTy o={o} />;
  }
};

const PrintFloatTy = ({ o }) => {
  return o.toLowerCase();
};

const PrintUintTy = ({ o }) => {
  return o.toLowerCase();
};

const PrintIntTy = ({ o }) => {
  return o.toLowerCase();
};
