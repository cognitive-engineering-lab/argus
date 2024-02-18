import {
  AliasTy,
  AliasTyKind,
  AssocItem,
  BoundTy,
  BoundVariable,
  Clause,
  CoroutineTyKind,
  CoroutineWitnessTyKind,
  DynamicTyKind,
  FloatTy,
  FnDef,
  FnSig,
  FnTrait,
  GenericArg,
  ImplHeader,
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
import _ from "lodash";
import React from "react";

import { Toggle } from "../../Toggle";
import { anyElems, fnInputsAndOutput, tyIsUnit } from "../../utilities/func";
import { PrintConst } from "./const";
import { PrintDefPath } from "./path";
import { PrintClause } from "./predicate";
import { Angled, CommaSeparated, DBraced, Kw, PlusSeparated } from "./syntax";
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

export const PrintImplHeader = ({ o }: { o: ImplHeader }) => {
  console.debug("Printing ImplHeader", o);
  const genArgs = _.map(o.args, arg => () => <PrintGenericArg o={arg} />);
  const argsWAngle =
    genArgs.length === 0 ? null : (
      <Angled>
        <Toggle
          summary={".."}
          Children={() => <CommaSeparated components={genArgs} />}
        />
      </Angled>
    );
  const trait = <PrintDefPath o={o.name} />;
  const selfTy = <PrintTy o={o.selfTy} />;
  const whereClause = (
    <PrintWhereClause
      predicates={o.predicates}
      tysWOBound={o.tysWithoutDefaultBounds}
    />
  );

  return (
    <>
      <Kw>impl</Kw>
      {argsWAngle} {trait} <Kw>for</Kw> {selfTy}
      {whereClause}
    </>
  );
};

export const PrintWhereClause = ({
  predicates,
  tysWOBound,
}: {
  predicates: any;
  tysWOBound: Ty[];
}) => {
  if (!anyElems(predicates.grouped, predicates.other, tysWOBound)) {
    return null;
  }

  const groupedClauses = _.map(predicates.grouped, (group, idx) => (
    <div className="WhereConstraint" key={idx}>
      <PrintClauseWithBounds o={group} />
    </div>
  ));
  const noGroupedClauses = _.map(predicates.other, (clause, idx) => (
    <div className="WhereConstraint" key={idx}>
      <PrintClause o={clause} />
    </div>
  ));

  const whereHoverContent = () => (
    <div className="DirNode WhereConstraintArea">
      {groupedClauses}
      {noGroupedClauses}
      {_.map(tysWOBound, (ty, idx) => (
        <div className="WhereConstraint" key={idx}>
          <PrintTy o={ty} />: ?Sized
        </div>
      ))}
    </div>
  );

  return (
    <>
      {" "}
      <Kw>where</Kw> <Toggle summary={".."} Children={whereHoverContent} />
    </>
  );
};

const PrintClauseWithBounds = ({ o }: { o: Clause }) => {
  const boundComponents = _.map(o.bounds, bound => () => (
    <>
      <PrintImplPolarity o={bound.polarity} />
      <PrintDefPath o={bound.path} />
    </>
  ));
  return (
    <>
      <PrintTy o={o.ty} />: <PlusSeparated components={boundComponents} />
    </>
  );
};

export const PrintTy = ({ o }: { o: Ty }) => {
  return <PrintTyKind o={o} />;
};

// TODO: enums that don't have an inner object need to use a
// comparison, instead of the `in` operator.
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
    console.debug("Printing Array", o);
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
    const [r, ty, mtbl] = o.Ref;
    const tyAndMut = {
      ty: ty,
      mutbl: mtbl,
    };
    return (
      <span>
        &<PrintRegion o={r} /> <PrintTypeAndMut o={tyAndMut} />
      </span>
    );
  } else if ("FnDef" in o) {
    return <PrintFnDef o={o.FnDef} />;
  } else if ("FnPtr" in o) {
    return <PrintPolyFnSig o={o.FnPtr} />;
  } else if ("Tuple" in o) {
    const components = _.map(o.Tuple, t => () => <PrintTy o={t} />);
    return (
      <span>
        (<CommaSeparated components={components} />)
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
    <>
      ({dynKind} {head} + {region})
    </>
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
        (<CommaSeparated components={inputComponents} />
        {variadic}){ret}
      </>
    );
  };

  const inner = (o: FnSig) => {
    const unsafetyStr = o.unsafety === "Unsafe" ? "unsafe " : null;
    // TODO: we could write the ABI here, or expose it at least.
    const abi = o.abi === "Rust" ? null : "extern ";
    const [inputs, output] = fnInputsAndOutput(o.inputs_and_output);
    return (
      <>
        fn <InnerSig inputs={inputs} output={output} cVariadic={o.c_variadic} />
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
      <Toggle summary=".." Children={() => <PrintPolyFnSig o={o.sig} />} />{" "}
      {"{"}
      <PrintDefPath o={o.path} />
      {"}"}
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
  switch (o.type) {
    case "IntVar":
      return "{{int}}";
    case "FloatVar":
      return "{{float}}";
    case "Named":
      // NOTE: we also have access to the symbol it came from o.name
      return <PrintDefPath o={o.pathDef} />;
    case "Unresolved":
      return "_";
    default:
      throw new Error("Unknown infer ty", o);
  }
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
  throw new Error("TODO");
};

const PrintImplPolarity = ({ o }: { o: ImplPolarity }) => {
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
      <span>
        ({o.kind}(<CommaSeparated components={args} />){ret})
      </span>
    );
  };

  const PrintAssocItem = ({ o }: { o: AssocItem }) => {
    console.debug("Printing AssocItem", o);
    return (
      <span>
        {o.name} = <PrintTerm o={o.term} />
      </span>
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
