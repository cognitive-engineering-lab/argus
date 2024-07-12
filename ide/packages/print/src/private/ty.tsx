import type {
  Abi,
  AliasTerm,
  AliasTy,
  AliasTyKind,
  AssocItem,
  BoundRegionKind,
  BoundTy,
  BoundTyKind,
  BoundVariable,
  BoundVariableKind,
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
import {
  anyElems,
  fnInputsAndOutput,
  isNamedBoundVariable,
  isNamedRegion,
  isUnitTy
} from "@argus/common/func";
import {} from "@floating-ui/react";
import _, { isObject } from "lodash";
import React, { useContext } from "react";
import { Toggle } from "../Toggle";
import { AllowProjectionSubst, ProjectionPathRender, TyCtxt } from "../context";
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
  nbsp
} from "./syntax";
import { PrintTerm } from "./term";

interface Binding<T> {
  value: T;
  boundVars: BoundVariableKind[];
}

export const PrintBinder = <T,>({
  binder,
  Child
}: {
  binder: Binding<T>;
  // FIXME: shouldn't this just be `React.FC<T>`?? Doesn't typecheck though...
  Child: React.FC<{ value: T }>;
}) => {
  const components = _.map(
    _.filter(binder.boundVars, isNamedBoundVariable),
    v => <PrintBoundVariableKind o={v} />
  );

  const b =
    components.length === 0 ? null : (
      <>
        for
        <Angled>
          <CommaSeparated components={components} />
        </Angled>
        {nbsp}
      </>
    );
  return (
    <>
      {b}
      <Child value={binder.value} />
    </>
  );
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
  const PrintCustomProjection = useContext(ProjectionPathRender);
  const tyCtx = useContext(TyCtxt)!;
  return (
    <PrintCustomProjection
      ctx={tyCtx}
      original={original}
      projection={projection}
    />
  );
};

export const PrintTyValue = ({ o }: { o: TyVal }) => {
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
  } else if ("Pat" in o) {
    const [ty] = o.Pat;
    return <PrintTy o={ty} />;
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
      mutbl: mtbl
    };

    if (!isNamedRegion(r) && mtbl === "Not") {
      return (
        <>
          &<PrintTy o={ty} />
        </>
      );
    }

    return (
      <>
        &<PrintRegion o={r} forceAnonymous={true} />{" "}
        <PrintTypeAndMut o={tyAndMut} />
      </>
    );
  } else if ("FnDef" in o) {
    return <PrintFnDef o={o.FnDef} />;
  } else if ("FnPtr" in o) {
    return <PrintPolyFnSig o={o.FnPtr} />;
  } else if ("Tuple" in o) {
    const components = _.map(o.Tuple, t => <PrintTy o={t} />);
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
  } else if ("CoroutineClosure" in o) {
    return <PrintCoroutineClosureTy o={o.CoroutineClosure} />;
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
  const components = _.map(o.autoTraits, t => <PrintDefPath o={t} />);
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
    const inputComponents = _.map(inputs, ty => <PrintTy o={ty} />);
    const variadic = !cVariadic ? null : inputs.length === 0 ? "..." : ", ...";
    const outVal = tyCtx.interner[output];
    const ret = isUnitTy(outVal) ? null : (
      <>
        {nbsp}
        {"->"}
        {nbsp}
        <PrintTyValue o={outVal} />
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

  const Inner = ({ value }: { value: FnSig }) => {
    const unsafetyStr = value.safety === "Unsafe" ? "unsafe " : null;
    const abi = <PrintAbi abi={value.abi} />;
    const [inputs, output] = fnInputsAndOutput(value.inputs_and_output);
    return (
      <>
        {unsafetyStr}
        {abi}fn
        <InnerSig
          inputs={inputs}
          output={output}
          cVariadic={value.c_variadic}
        />
      </>
    );
  };

  return <PrintBinder binder={o} Child={Inner} />;
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

export const PrintRegion = ({
  o,
  forceAnonymous = false
}: { o: Region; forceAnonymous?: boolean }) => {
  switch (o.type) {
    case "Static": {
      return "'static";
    }
    case "Named": {
      return <PrintSymbol o={o.data} />;
    }
    case "Anonymous": {
      // NOTE: by default we don't print anonymous lifetimes. There are times
      // when it looks better, e.g., when the region is `mut`. One gotcha right now
      // is that we don't rename them, which makes reasoning about anonymouse lifetimes
      // tricky.
      if (forceAnonymous) {
        return "'_";
      }
      return null;
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

export const PrintBoundTyKind = ({ o }: { o: BoundTyKind }) => {
  if ("Anon" === o) {
    return null;
  } else if ("Param" in o) {
    const [name] = o.Param;
    return <PrintSymbol o={name} />;
  }

  throw new Error("Unknown bound ty kind", o);
};

export const PrintBoundVariableKind = ({ o }: { o: BoundVariableKind }) => {
  if ("Const" === o) {
    // TODO: not sure what to do with boudn "consts", we don't have data for them.
    return null;
  } else if ("Ty" in o) {
    return <PrintBoundTyKind o={o.Ty} />;
  } else if ("Region" in o) {
    return <PrintBoundRegionKind o={o.Region} />;
  }

  throw new Error("Unknown bound variable kind", o);
};

export const PrintBoundRegionKind = ({ o }: { o: BoundRegionKind }) => {
  // TODO: what do we do in these cases?
  if ("BrAnon" === o) {
    return null;
  } else if ("BrEnv" === o) {
    return null;
  }
  if ("BrNamed" in o && o.BrNamed[0] !== "'_") {
    const [name] = o.BrNamed;
    return <PrintSymbol o={name} />;
  }
};

export const PrintPolarity = ({ o }: { o: Polarity }) => {
  return o === "Negative" ? "!" : o === "Maybe" ? "?" : null;
};

export const PrintOpaqueImplType = ({ o }: { o: OpaqueImpl }) => {
  console.debug("Printing OpaqueImplType", o);

  const PrintFnTrait = ({ o }: { o: FnTrait }) => {
    const args = _.map(o.params, param => <PrintTy o={param} />);
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
    const ownArgs = _.map(o.ownArgs, arg => <PrintGenericArg o={arg} />);
    const assocArgs = _.map(o.assocArgs, arg => <PrintAssocItem o={arg} />);
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

  const fnTraits = _.map(o.fnTraits, trait => <PrintFnTrait o={trait} />);
  const traits = _.map(o.traits, trait => <PrintTrait o={trait} />);
  const lifetimes = _.map(o.lifetimes, lifetime => (
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
