import React, { useState } from "react";

export const PrettyObligation = ({ obligation }) => {
  console.debug("Printing Obligation", obligation);
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

const PrintDefPath = ({ o }) => {
  // TODO: change in the future
  if (!(typeof o === "string")) {
    throw new Error("Expected string", o);
  }

  return <span>{o}</span>;
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
  return <PrintTyKind o={o} />;
};

const PrintTyKind = ({ o }) => {
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
    throw new Error("TODO");
  } else if ("Slice" in o) {
    throw new Error("TODO");
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
    throw new Error("TODO");
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
