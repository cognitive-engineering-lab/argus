import {
  Abi,
  AliasTy,
  AliasTyKind,
  AssocItem,
  BoundTy,
  BoundVariable,
  CoroutineTyKind,
  CoroutineWitnessTyKind,
  DynamicTyKind,
  FloatTy,
  FnDef,
  FnSig,
  FnTrait,
  GenericArg,
  ImplPolarity,
  InferTy,
  IntTy,
  OpaqueImpl,
  ParamTy,
  PlaceholderBoundTy,
  PolyExistentialPredicates,
  PolyFnSig,
  Region,
  Symbol,
  Trait,
  Ty,
  TyKind,
  TypeAndMut,
  UintTy,
} from "@argus/common/bindings";
import _, { isObject } from "lodash";
import React from "react";

import { Toggle } from "../../Toggle";
import { anyElems, fnInputsAndOutput, tyIsUnit } from "../../utilities/func";
import { PrintConst } from "./const";
import { PrintDefPath } from "./path";
import {
  Angled,
  CommaSeparated,
  DBraced,
  Parenthesized,
  Placeholder,
  PlusSeparated,
  SqBraced,
} from "./syntax";
import { PrintTerm } from "./term";

export const PrintBinder = ({
  binder,
  innerF,
}: {
  binder: any;
  innerF: any;
}) => {
  return innerF(binder.value);
};

export const PrintTy = ({ o }: { o: Ty }) => {
  return <PrintTyKind o={o} />;
};

export const PrintTyKind = ({ o }: { o: TyKind }) => {
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
      <SqBraced>
        <PrintTy o={ty} />; <PrintConst o={sz} />
      </SqBraced>
    );
  } else if ("Slice" in o) {
    return (
      <SqBraced>
        <PrintTy o={o.Slice} />
      </SqBraced>
    );
  } else if ("RawPtr" in o) {
    const m = o.RawPtr.mutbl === "Not" ? "const" : "mut";
    return (
      <>
        *{m} <PrintTy o={o.RawPtr.ty} />
      </>
    );
  } else if ("Ref" in o) {
    const [r, ty, mtbl] = o.Ref;
    const tyAndMut = {
      ty: ty,
      mutbl: mtbl,
    };
    return (
      <>
        &<PrintRegion o={r} /> <PrintTypeAndMut o={tyAndMut} />
      </>
    );
  } else if ("FnDef" in o) {
    return <PrintFnDef o={o.FnDef} />;
  } else if ("FnPtr" in o) {
    return <PrintPolyFnSig o={o.FnPtr} />;
  } else if ("Tuple" in o) {
    const components = _.map(o.Tuple, t => () => <PrintTy o={t} />);
    return (
      <Parenthesized>
        <CommaSeparated components={components} />
      </Parenthesized>
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

export const PrintCoroutineTy = ({ o }: { o: CoroutineTyKind }) => {
  const movability =
    o.shouldPrintMovability && o.movability === "Static" ? "static " : null;
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

export const PrintCoroutineWitnessTy = ({
  o,
}: {
  o: CoroutineWitnessTyKind;
}) => {
  return (
    <DBraced>
      <PrintDefPath o={o} />
    </DBraced>
  );
};

export const PrintPolyExistentialPredicates = ({
  o,
}: {
  o: PolyExistentialPredicates;
}) => {
  const head = o.data === undefined ? null : <PrintDefPath o={o.data} />;
  const components = _.map(o.autoTraits, t => () => <PrintDefPath o={t} />);
  return (
    <>
      {head}
      <CommaSeparated components={components} />
    </>
  );
};

export const PrintDynamicTy = ({ o }: { o: DynamicTyKind }) => {
  const head = <PrintPolyExistentialPredicates o={o.predicates} />;
  const region = <PrintRegion o={o.region} />;
  const dynKind = o.kind === "Dyn" ? "dyn" : "dyn*";
  return (
    <Parenthesized>
      {dynKind} {head} + {region}
    </Parenthesized>
  );
};

export const PrintAliasTyKind = ({ o }: { o: AliasTyKind }) => {
  switch (o.type) {
    case "OpaqueImpl": {
      return <PrintOpaqueImplType o={o.data} />;
    }
    case "AliasTy": {
      return <PrintAliasTy o={o.data} />;
    }
    case "DefPath": {
      return <PrintDefPath o={o.data} />;
    }
    default: {
      throw new Error("Unknown alias ty kind", o);
    }
  }
};

export const PrintAliasTy = ({ o }: { o: AliasTy }) => {
  switch (o.type) {
    case "PathDef":
      return <PrintDefPath o={o.data} />;
    case "Inherent":
      return <PrintDefPath o={o.data} />;
    default:
      throw new Error("Unknown alias ty kind", o);
  }
};

export const PrintPolyFnSig = ({ o }: { o: PolyFnSig }) => {
  const InnerSig = ({
    inputs,
    output,
    cVariadic,
  }: {
    inputs: Ty[];
    output: Ty;
    cVariadic: boolean;
  }) => {
    const inputComponents = _.map(inputs, ty => () => <PrintTy o={ty} />);
    const variadic = !cVariadic ? null : inputs.length === 0 ? "..." : ", ...";
    const ret = tyIsUnit(output) ? null : (
      <>
        {" "}
        {"->"} <PrintTy o={output} />
      </>
    );
    return (
      <>
        <Parenthesized>
          <CommaSeparated components={inputComponents} />
          {variadic}
        </Parenthesized>
        {ret}
      </>
    );
  };

  const PrintAbi = ({ abi }: { abi: Abi }) => {
    if (abi === "Rust") {
      return null;
    }

    const propertyAbis = [
      "C",
      "Cdecl",
      "Stdcall",
      "Fastcall",
      "Vectorcall",
      "Thiscall",
      "Aapcs",
      "Win64",
      "SysV64",
      "System",
    ];

    const fromString = (abi: string) => {
      return `extern ${abi.toLowerCase()} `;
    };

    if (isObject(abi)) {
      for (const prop in abi) {
        if (propertyAbis.includes(prop)) {
          return fromString(prop);
        }
      }
    } else {
      return fromString(abi);
    }
  };

  const inner = (o: FnSig) => {
    const unsafetyStr = o.unsafety === "Unsafe" ? "unsafe " : null;
    const abi = <PrintAbi abi={o.abi} />;
    const [inputs, output] = fnInputsAndOutput(o.inputs_and_output);
    return (
      <>
        {unsafetyStr}
        {abi}fn{" "}
        <InnerSig inputs={inputs} output={output} cVariadic={o.c_variadic} />
      </>
    );
  };

  return <PrintBinder binder={o} innerF={inner} />;
};

export const PrintFnDef = ({ o }: { o: FnDef }) => {
  // NOTE: `FnDef`s have both a path and a signature.
  // We should show both (somehow), not sure what's the best way to present it.
  return (
    <>
      <PrintDefPath o={o.path} />{" "}
      <Toggle summary=".." Children={() => <PrintPolyFnSig o={o.sig} />} />
    </>
  );
};

export const PrintParamTy = ({ o }: { o: ParamTy }) => {
  return <PrintSymbol o={o.name} />;
};

export const PrintSymbol = ({ o }: { o: Symbol }) => {
  return o;
};

export const PrintBoundTy = ({ o }: { o: BoundTy }) => {
  switch (o.type) {
    case "Named": {
      return <PrintSymbol o={o.data} />;
    }
    case "Bound": {
      return <PrintBoundVariable o={o.data} />;
    }
    default: {
      throw new Error("Unknown bound ty kind", o);
    }
  }
};

export const PrintPlaceholderTy = ({ o }: { o: PlaceholderBoundTy }) => {
  switch (o.type) {
    case "Anon": {
      // TODO: what do we really want to anon placeholders?
      return "{anon}";
    }
    case "Named": {
      return <PrintSymbol o={o.data} />;
    }
  }
};

export const PrintInferTy = ({ o }: { o: InferTy }) => {
  const Inner =
    o === "IntVar"
      ? () => <DBraced>int</DBraced>
      : o === "FloatVar"
      ? () => <DBraced>float</DBraced>
      : o === "Unresolved"
      ? () => "_"
      : "Named" in o
      ? () => <PrintDefPath o={o.Named[1]} />
      : "Unnamed" in o
      ? () => <PrintDefPath o={o.Unnamed} />
      : "SourceInfo" in o
      ? () => <code>{o.SourceInfo}</code>
      : () => {
          throw new Error("Unknown infer ty", o);
        };

  return (
    <Placeholder>
      <Inner />
    </Placeholder>
  );
};

export const PrintTypeAndMut = ({ o }: { o: TypeAndMut }) => {
  return (
    <>
      {o.mutbl === "Mut" ? "mut " : null}
      <PrintTy o={o.ty} />
    </>
  );
};

export const PrintGenericArg = ({ o }: { o: GenericArg }) => {
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

export const PrintRegion = ({ o }: { o: Region }) => {
  switch (o.type) {
    case "Static": {
      return "'static";
    }
    case "Named": {
      return <PrintSymbol o={o.data} />;
    }
    case "Anonymous": {
      // TODO: maybe we don't want to print anonymous lifetimes?
      return "'_";
    }
    default: {
      throw new Error("Unknown region type", o);
    }
  }
};

// --------------------------
// Numeric types

export const PrintFloatTy = ({ o }: { o: FloatTy }) => {
  return o.toLowerCase();
};

export const PrintUintTy = ({ o }: { o: UintTy }) => {
  return o.toLowerCase();
};

export const PrintIntTy = ({ o }: { o: IntTy }) => {
  return o.toLowerCase();
};

export const PrintBoundVariable = ({ o }: { o: BoundVariable }) => {
  if ("Error" in o) {
    return <DBraced>{o.Error}</DBraced>;
  } else {
    throw new Error("Unknown bound variable", o);
  }
};

export const PrintImplPolarity = ({ o }: { o: ImplPolarity }) => {
  return o === "Negative" ? "!" : null;
};

export const PrintOpaqueImplType = ({ o }: { o: OpaqueImpl }) => {
  console.debug("Printing OpaqueImplType", o);

  const PrintFnTrait = ({ o }: { o: FnTrait }) => {
    const args = _.map(o.params, param => () => <PrintTy o={param} />);
    const ret =
      o.retTy !== undefined ? (
        <>
          {" -> "}
          <PrintTy o={o.retTy} />
        </>
      ) : null;
    return (
      <Parenthesized>
        {o.kind}
        <Parenthesized>
          <CommaSeparated components={args} />
        </Parenthesized>
        {ret}
      </Parenthesized>
    );
  };

  const PrintAssocItem = ({ o }: { o: AssocItem }) => {
    console.debug("Printing AssocItem", o);
    return (
      <>
        {o.name} = <PrintTerm o={o.term} />
      </>
    );
  };

  const PrintTrait = ({ o }: { o: Trait }) => {
    console.debug("Printing Trait", o);
    const prefix = <PrintImplPolarity o={o.polarity} />;
    const name = <PrintDefPath o={o.traitName} />;
    const ownArgs = _.map(o.ownArgs, arg => () => <PrintGenericArg o={arg} />);
    const assocArgs = _.map(o.assocArgs, arg => () => (
      <PrintAssocItem o={arg} />
    ));
    const argComponents = [...ownArgs, ...assocArgs];
    const list =
      argComponents.length === 0 ? null : (
        <Angled>
          <CommaSeparated components={argComponents} />
        </Angled>
      );
    return (
      <>
        {prefix}
        {name}
        {list}
      </>
    );
  };

  const fnTraits = _.map(o.fnTraits, trait => () => <PrintFnTrait o={trait} />);
  const traits = _.map(o.traits, trait => () => <PrintTrait o={trait} />);
  const lifetimes = _.map(o.lifetimes, lifetime => () => (
    <PrintRegion o={lifetime} />
  ));
  const implComponents = _.concat(fnTraits, traits, lifetimes);

  const pe = anyElems(implComponents);
  const addSized = o.hasSizedBound && (!pe || o.hasNegativeSizedBound);
  const addMaybeSized = !o.hasSizedBound && !o.hasNegativeSizedBound;
  const sized =
    addSized || addMaybeSized
      ? `${pe ? " + " : ""}${addMaybeSized ? "?" : ""}Sized`
      : null;

  const start =
    implComponents.length === 0 && sized === "" ? "{opaque}" : "impl ";

  return (
    <>
      {start}
      <PlusSeparated components={implComponents} />
      {sized}
    </>
  );
};
