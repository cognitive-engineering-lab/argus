import React from "react";

import { PrintConst } from "./const";
import { PrintDefPath } from "./path";
import { fnInputsAndOutput, intersperse, tyIsUnit } from "./utilities";

export const PrintBinder = ({ binder, innerF }) => {
  return innerF(binder.value);
};

export const Angled = ({ children }) => {
  return (
    <span>
      {"<"}
      {children}
      {">"}
    </span>
  );
};

export const DBraced = ({ children }) => {
  return (
    <span>
      {"{{"}
      {children}
      {"}}"}
    </span>
  );
};

export const PrintTy = ({ o }) => {
  console.debug("Printing Ty", o);
  return <PrintTyKind o={o} />;
};

export const PrintFnSig = ({ inputs, output, cVariadic }) => {
  const doTy = (ty, i) => {
    return <PrintTy o={ty} key={i} />;
  };
  const variadic = !cVariadic ? "" : args.length === 0 ? "..." : ", ...";
  const ret = tyIsUnit(output) ? (
    ""
  ) : (
    <span>
      {" "}
      {"->"} <PrintTy o={output} />
    </span>
  );
  return (
    <span>
      ({intersperse(inputs, ", ", doTy)}
      {variadic}){ret}
    </span>
  );
};

// TODO: enums that don't have an inner object need to use a
// comparison, instead of the `in` operator.
export const PrintTyKind = ({ o }) => {
  if ("Bool" === o) {
    return "bool";
  } else if ("Char" === o) {
    return "char";
  } else if ("Str" === o) {
    return "str";
  } else if ("Never" === o) {
    return "!";
  } else if ("Error" === o) {
    return "{error}";
  } else if ("Int" in o) {
    return <PrintIntTy o={o.Int} />;
  } else if ("Uint" in o) {
    return <PrintUintTy o={o.Uint} />;
  } else if ("Float" in o) {
    return <PrintFloatTy o={o.Float} />;
  } else if ("Adt" in o) {
    return <PrintDefPath o={o.Adt} />;
  } else if ("Array" in o) {
    const [ty, sz] = o.Array;
    return (
      <span>
        [<PrintTy o={ty} />; <PrintConst o={sz} />]
      </span>
    );
  } else if ("Slice" in o) {
    return (
      <span>
        [<PrintTy o={o.Slice} />]
      </span>
    );
  } else if ("RawPtr" in o) {
    const m = o.RawPtr.mutbl === "Not" ? "const" : "mut";
    return (
      <span>
        *{m} <PrintTy o={o.RawPtr.ty} />
      </span>
    );
  } else if ("Ref" in o) {
    // TODO: when to print regions?
    const [r, ty, mtbl] = o.Ref;
    const tyAndMut = {
      ty: ty,
      mutbl: mtbl,
    };
    return (
      <span>
        &<PrintTypeAndMut o={tyAndMut} />
      </span>
    );
  } else if ("FnDef" in o) {
    return <PrintFnDef o={o.FnDef} />;
  } else if ("FnPtr" in o) {
    const binderFnSig = o.FnPtr;

    const inner = o => {
      const unsafetyStr = o.unsafety === "Unsafe" ? "unsafe " : "";
      // TODO: we could write the ABI here, or expose it at least.
      const abi = o.abi === "Rust" ? "" : "extern ";
      const [inputs, output] = fnInputsAndOutput(o.inputs_and_output);
      return (
        <span>
          fn{" "}
          <PrintFnSig
            inputs={inputs}
            output={output}
            cVariadic={o.c_variadic}
          />
        </span>
      );
    };

    return <PrintBinder binder={binderFnSig} innerF={inner} />;
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
    return <PrintPlaceholderTy o={o.Placeholder} />;
  } else if ("Infer" in o) {
    return <PrintInferTy o={o.Infer} />;
  } else if ("Foreign" in o) {
    return <PrintDefPath o={o.Foreign} />;
  } else if ("Closure" in o) {
    return <PrintDefPath o={o.Closure} />;
  } else if ("Param" in o) {
    return <PrintParamTy o={o.Param} />;
  } else if ("Bound" in o) {
    return <PrintBoundTy o={o.Bound} />;
  } else if ("Alias" in o) {
    return <PrintAliasTyKind o={o.Alias} />;
  } else if ("Dynamic" in o) {
    return <PrintDynamicTy o={o.Dynamic} />;
  } else if ("Coroutine" in o) {
    return <PrintCoroutineTy o={o.Coroutine} />;
  } else if ("CoroutineWitness" in o) {
    return <PrintCoroutineWitnessTy o={o.CoroutineWitness} />;
  } else {
    throw new Error("Unknown ty kind", o);
  }
};

export const PrintCoroutineTy = ({ o }) => {
  const movability =
    o.shouldPrintMovability && o.movability === "Static" ? "static " : "";
  const pathDef = <PrintDefPath o={o.path} />;
  // NOTE: the upvars are tupled together into a single type.
  const upvars = <PrintTy o={o.upvarTys} />;
  const witness = <PrintTy o={o.witness} />;
  // TODO: we can probably move the upvars and witness into a hidden div
  return (
    <DBraced>
      {movability}
      {pathDef} upvar_tys={upvars} witness={witness}
    </DBraced>
  );
};

export const PrintCoroutineWitnessTy = ({ o }) => {
  return (
    <DBraced>
      <PrintDefPath o={o} />
    </DBraced>
  );
};

export const PrintDynamicTy = ({ o }) => {
  const hasHead = o.data !== undefined;
  const head = hasHead ? <PrintDefPath o={o.data} /> : "";
  const autoTraits = _.map(o.autoTraits, (trait, i) => () => (
    <PrintDefPath o={trait} key={i} />
  ));
  return (
    <span>
      {head}
      {_.map(autoTraits, (Trait, i) => (
        <span key={i}>
          {hasHead || i > 0 ? " + " : ""}
          <Trait />
        </span>
      ))}
    </span>
  );
};

export const PrintAliasTyKind = ({ o }) => {
  switch (o.type) {
    case "opaqueImplType": {
      return <PrintOpaqueImplType o={o.data} />;
    }
    case "aliasTy": {
      return <PrintAliasTy o={o.data} />;
    }
    case "defPath": {
      return <PrintDefPath o={o.data} />;
    }
    default: {
      throw new Error("Unknown alias ty kind", o);
    }
  }
};

export const PrintAliasTy = ({ o }) => {
  // TODO: this isn't quite accurate, the serialization code needs to get
  // updated, see AliasTyDef::serialize for up-to-date info.
  // The case for 'inherent projections' is missing.
  return <PrintDefPath o={o} />;
};

export const PrintOqaqueImplType = ({ o }) => {
  throw new Error("NYI");
};

export const PrintFnDef = ({ o }) => {
  // NOTE: `FnDef`s have both a path and a signature.
  // We should show both (somehow), not sure what's the best way to present it.
  return <PrintDefPath o={o.path} />;
};

export const PrintParamTy = ({ o }) => {
  return <PrintSymbol o={o.name} />;
};

export const PrintSymbol = ({ o }) => {
  return o;
};

export const PrintBoundTy = ({ o }) => {
  switch (o.type) {
    case "named": {
      return <span>{o.data}</span>;
    }
    default: {
      throw new Error("Unknown bound ty kind", o);
    }
  }
};

export const PrintPlaceholderTy = ({ o }) => {
  switch (o.type) {
    case "anon": {
      // TODO: what do we really want to anon placeholders?
      return "{anon}";
    }
    case "named": {
      return <span>{o.data}</span>;
    }
  }
};

// TODO: infer types are not yet "done"
export const PrintInferTy = ({ o }) => {
  if (o === "Unresolved") {
    return "_";
  } else {
    throw new Error("Unknown infer ty", o);
  }
};

export const PrintTyVar = ({ o }) => {
  if (typeof o === "string" || o instanceof String) {
    return o;
  } else {
    return <PrintTy o={o} />;
  }
};

export const PrintTypeAndMut = ({ o }) => {
  return (
    <span>
      {o.mutbl === "Mut" ? "mut " : ""}
      <PrintTy o={o.ty} />
    </span>
  );
};

export const PrintGenericArg = ({ o }) => {
  console.debug("GenericArg", o);

  if ("Type" in o) {
    return <PrintTy o={o.Type} />;
  } else if ("Lifetime" in o) {
    return <PrintRegion o={o.Lifetime} />;
  } else if ("Const" in o) {
    return <PrintConst o={o.Const} />;
  } else {
    throw new Error("Unknown generic arg", o);
  }
};

export const PrintRegion = ({ o }) => {
  switch (o.type) {
    case "static": {
      return "'static";
    }
    case "named": {
      return `'${o.data}`;
    }
    case "anonymous": {
      // TODO: maybe we don't want to print anonymous lifetimes?
      return "'_";
    }
    case "vid": {
      return "";
    }
    default: {
      throw new Error("Unknown region type", o);
    }
  }
};

// --------------------------
// Numeric types

export const PrintFloatTy = ({ o }) => {
  return o.toLowerCase();
};

export const PrintUintTy = ({ o }) => {
  return o.toLowerCase();
};

export const PrintIntTy = ({ o }) => {
  return o.toLowerCase();
};

export const PrintBoundVariable = ({ o }) => {
  throw new Error("TODO");
};

export const PrintOpaqueImplType = ({ o }) => {
  const areElems = (...lists) => _.some(lists, l => l.length > 0);

  const PrintFnTrait = ({ o }) => {
    const args = _.map(o.params, (param, i) => <PrintTy o={param} key={i} />);
    const ret =
      o.retTy !== undefined ? (
        <span>
          {" -> "}
          <PrintTy o={o.retTy} />
        </span>
      ) : (
        ""
      );
    return (
      <span>
        ({o.kind}({args}){ret})
      </span>
    );
  };

  const PrintAssocItem = ({ o }) => {
    return (
      <span>
        o.name = <PrintTerm o={o.term} />
      </span>
    );
  };

  const PrintTrait = ({ o }) => {
    const prefix = o.polarity === "Negative" ? "!" : "";
    const name = <PrintDefPath o={o.traitName} />;
    const args = _.map(o.ownArgs, (arg, i) => () => (
      <PrintGenericArg o={arg} key={i} />
    ));
    const assocArgs = _.map(o.assocArgs, (arg, i) => () => (
      <PrintAssocItem o={arg} key={i} />
    ));
    const list = areElems(args, assocArgs) ? (
      <Angled>
        {intersperse(args.concat(assocArgs), ", ", E => (
          <E />
        ))}
      </Angled>
    ) : (
      ""
    );
    return (
      <span>
        {prefix}
        {name}
        {list}
      </span>
    );
  };

  const fnTraits = _.map(o.fnTraits, (trait, i) => (
    <PrintFnTrait o={trait} key={i} />
  ));
  const traits = _.map(o.traits, (trait, i) => (
    <PrintTrait o={trait} key={i} />
  ));
  const lifetimes = _.map(o.lifetimes, (lifetime, i) => (
    <PrintRegion o={lifetime} key={i} />
  ));
  const sized = "TODO: ?Sized";

  const firstSep = areElems(fnTraits) && traits.length > 0 ? " + " : "";
  const secondSep =
    areElems(fnTraits, traits) && lifetimes.length > 0 ? " + " : "";
  const thirdSep =
    areElems(fnTraits, traits, lifetimes) && sized !== "" ? " + " : "";

  return (
    <span>
      impl {fnTraits}
      {firstSep}
      {traits}
      {secondSep}
      {lifetimes}
      {thirdSep}
      {sized}
    </span>
  );
};
