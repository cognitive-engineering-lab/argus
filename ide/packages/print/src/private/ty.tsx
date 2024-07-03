import type {
  Abi,
  AliasTerm,
  AliasTy,
  AliasTyKind,
  AssocItem,
  BoundTy,
  BoundVariable,
  CoroutineClosureTyKind,
  CoroutineTyKind,
  CoroutineWitnessTyKind,
  DynamicTyKind,
  FloatTy,
  FnDef,
  FnSig,
  FnTrait,
  GenericArg,
  InferTy,
  IntTy,
  OpaqueImpl,
  ParamTy,
  PlaceholderBoundTy,
  Polarity,
  PolyExistentialPredicates,
  PolyFnSig,
  Region,
  // biome-ignore lint: lint/suspicious/noShadowRestrictedNames
  Symbol,
  Trait,
  Ty,
  TyKind,
  TyVal,
  TypeAndMut,
  UintTy
} from "@argus/common/bindings";
import { anyElems, fnInputsAndOutput, tyIsUnit } from "@argus/common/func";
import {} from "@floating-ui/react";
import _, { isObject } from "lodash";
import React, { useContext } from "react";
import Comment from "../Comment";
import Indented from "../Indented";
import { Toggle } from "../Toggle";
import { AllowPathTrim, AllowProjectionSubst, TyCtxt } from "../context";
import { PrintConst } from "./const";
import { PrintDefPath } from "./path";
import {
  Angled,
  CommaSeparated,
  DBraced,
  Parenthesized,
  Placeholder,
  PlusSeparated,
  SqBraced
} from "./syntax";
import { PrintTerm } from "./term";

export const PrintBinder = ({
  binder,
  innerF
}: {
  binder: any;
  innerF: any;
}) => {
  return innerF(binder.value);
};

export const PrintTy = ({ o }: { o: Ty }) => {
  const tyCtx = useContext(TyCtxt);
  if (tyCtx === undefined) {
    throw new Error("Ty Interner not set");
  }

  const tyVal = tyCtx.interner[o];
  const projectedId = tyCtx.projections[o];
  const allowProjection = useContext(AllowProjectionSubst);
  if (allowProjection && projectedId !== undefined) {
    const projectedValue = tyCtx.interner[projectedId];
    console.info("Printing alias instead!");
    return <PrintTyProjected original={tyVal} projection={projectedValue} />;
  }

  return <PrintTyValue o={tyVal} />;
};

export const PrintTyProjected = ({
  original,
  projection
}: { original: TyVal; projection: TyVal }) => {
  const Content = (
    <AllowPathTrim.Provider value={false}>
      <AllowProjectionSubst.Provider value={false}>
        <p> This type is from a projection:</p>
        <p>Projected type:</p>
        <Indented>
          <PrintTyValue o={projection} />
        </Indented>
        <p>Full path:</p>
        <Indented>
          <PrintTyValue o={original} />
        </Indented>
      </AllowProjectionSubst.Provider>
    </AllowPathTrim.Provider>
  );

  return <Comment Child={<PrintTyValue o={projection} />} Content={Content} />;
};

export const PrintTyValue = ({ o }: { o: TyVal }) => {
  return <PrintTyKind o={o} />;
};

export const PrintTyKind = ({ o }: { o: TyKind }) => {
  if ("Bool" === o) {
    return "bool";
  }
  if ("Char" === o) {
    return "char";
  }
  if ("Str" === o) {
    return "str";
  }
  if ("Never" === o) {
    return "!";
  }
  if ("Error" === o) {
    return "{error}";
  }
  if ("Int" in o) {
    return <PrintIntTy o={o.Int} />;
  }
  if ("Uint" in o) {
    return <PrintUintTy o={o.Uint} />;
  }
  if ("Float" in o) {
    return <PrintFloatTy o={o.Float} />;
  }
  if ("Pat" in o) {
    const [ty] = o.Pat;
    return <PrintTy o={ty} />;
  }
  if ("Adt" in o) {
    return <PrintDefPath o={o.Adt} />;
  }
  if ("Array" in o) {
    const [ty, sz] = o.Array;
    return (
      <SqBraced>
        <PrintTy o={ty} />; <PrintConst o={sz} />
      </SqBraced>
    );
  }
  if ("Slice" in o) {
    return (
      <SqBraced>
        <PrintTy o={o.Slice} />
      </SqBraced>
    );
  }
  if ("RawPtr" in o) {
    const m = o.RawPtr.mutbl === "Not" ? "const" : "mut";
    return (
      <>
        *{m} <PrintTy o={o.RawPtr.ty} />
      </>
    );
  }
  if ("Ref" in o) {
    const [r, ty, mtbl] = o.Ref;
    const tyAndMut = {
      ty: ty,
      mutbl: mtbl
    };
    return (
      <>
        &<PrintRegion o={r} /> <PrintTypeAndMut o={tyAndMut} />
      </>
    );
  }
  if ("FnDef" in o) {
    return <PrintFnDef o={o.FnDef} />;
  }
  if ("FnPtr" in o) {
    return <PrintPolyFnSig o={o.FnPtr} />;
  }
  if ("Tuple" in o) {
    const components = _.map(o.Tuple, t => () => <PrintTy o={t} />);
    return (
      <Parenthesized>
        <CommaSeparated components={components} />
      </Parenthesized>
    );
  }
  if ("Placeholder" in o) {
    return <PrintPlaceholderTy o={o.Placeholder} />;
  }
  if ("Infer" in o) {
    return <PrintInferTy o={o.Infer} />;
  }
  if ("Foreign" in o) {
    return <PrintDefPath o={o.Foreign} />;
  }
  if ("Closure" in o) {
    return <PrintDefPath o={o.Closure} />;
  }
  if ("CoroutineClosure" in o) {
    return <PrintCoroutineClosureTy o={o.CoroutineClosure} />;
  }
  if ("Param" in o) {
    return <PrintParamTy o={o.Param} />;
  }
  if ("Bound" in o) {
    return <PrintBoundTy o={o.Bound} />;
  }
  if ("Alias" in o) {
    return <PrintAliasTyKind o={o.Alias} />;
  }
  if ("Dynamic" in o) {
    return <PrintDynamicTy o={o.Dynamic} />;
  }
  if ("Coroutine" in o) {
    return <PrintCoroutineTy o={o.Coroutine} />;
  }
  if ("CoroutineWitness" in o) {
    return <PrintCoroutineWitnessTy o={o.CoroutineWitness} />;
  }
  throw new Error("Unknown ty kind", o);
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

export const PrintCoroutineClosureTy = ({
  o
}: {
  o: CoroutineClosureTyKind;
}) => {
  // TODO: we can print other things known to the closure, like kind, signature, upvars, etc.
  return <PrintDefPath o={o.path} />;
};

export const PrintCoroutineWitnessTy = ({
  o
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
  o
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

export const PrintAliasTerm = ({ o }: { o: AliasTerm }) => {
  return <PrintDefPath o={o} />;
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
    cVariadic
  }: {
    inputs: Ty[];
    output: Ty;
    cVariadic: boolean;
  }) => {
    const tyCtx = useContext(TyCtxt)!;
    const inputComponents = _.map(inputs, ty => () => <PrintTy o={ty} />);
    const variadic = !cVariadic ? null : inputs.length === 0 ? "..." : ", ...";
    const outVal = tyCtx.interner[output];
    const ret = tyIsUnit(outVal) ? null : (
      <>
        {" "}
        {"->"} <PrintTyValue o={outVal} />
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
      "System"
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
    const unsafetyStr = o.safety === "Unsafe" ? "unsafe " : null;
    const abi = <PrintAbi abi={o.abi} />;
    const [inputs, output] = fnInputsAndOutput(o.inputs_and_output);
    return (
      <>
        {unsafetyStr}
        {abi}fn
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
  }
  if ("Lifetime" in o) {
    return <PrintRegion o={o.Lifetime} />;
  }
  if ("Const" in o) {
    return <PrintConst o={o.Const} />;
  }
  throw new Error("Unknown generic arg", o);
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
  }
  throw new Error("Unknown bound variable", o);
};

export const PrintPolarity = ({ o }: { o: Polarity }) => {
  return o === "Negative" ? "!" : o === "Maybe" ? "?" : null;
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
    const prefix = <PrintPolarity o={o.polarity} />;
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
