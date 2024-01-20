import React from "react";

import { PrintConst } from "./const";
import { PrintDefPath } from "./path";
import { fnInputsAndOutput, intersperse, tyIsUnit } from "./utilities";

export const PrintBinder = ({ binder, innerF }) => {
  return innerF(binder.value);
};

export const PrintTy = ({ o }) => {
  console.log("Printing Ty", o);
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
  } else {
    throw new Error("Unknown ty kind", o);
  }
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
  throw new Error("TODO");
};

export const PrintPlaceholderTy = ({ o }) => {
  switch (o.bound.kind) {
    case "Anon": {
      // TODO: what do we really want to anon placeholders?
      return "{anon}";
    }
    case "Param": {
      // TODO: do we want to use the `path` here?
      const [path, name] = o.bound.kind.Param;
      return <span>{name}</span>;
    }
  }
};

export const PrintInferTy = ({ o }) => {
  if (o === "Unresolved") {
    return "???";
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
  console.log("GenericArg", o);

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
