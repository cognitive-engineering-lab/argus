import React, { useState } from "react";


export function printObligation(o) {
    console.debug("Printing Obligation", o);
    return printBinderPredicateKind(o.data);
}

function printBinder(o, innerF) {
    return innerF(o.value);
}

function printBinderPredicateKind(o) {
    return printBinder(o, printPredicateKind);
}

function printPredicateKind(o) {
    if ("Clause" in o) {
        return printClauseKind(o.Clause);
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
}

function printClauseKind(o) {
    if ("Trait" in o) {
        return printTraitPredicate(o.Trait);
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
}

function printTraitPredicate(o) {
    let polarity = o.polarity === "Negative" ? "!" : "";
    return (
        <>
            <span>{polarity}</span>
            <span>{printTraitRef(o.trait_ref)}</span>
        </>
    );
}

function printTraitRef(o) {
    return (
        <span>
            <span>{printTy(o.self_ty)}</span>
            as
            <span>{printDefPath(o.trait_path)}</span>
        </span>
    );
}

function printDefPath(o) {
    let [isHover, setIsHover] = useState(false);
    let content = isHover ? printDefPathData(o.data) : printShortDefPath(o);
    let toggleHover = () => setIsHover(!isHover);
    return (
        <span onMouseEnter={toggleHover} onMouseLeave={toggleHover}>{content}</span>
    );
}

function printShortDefPath(o) {
    return printDefPathData(_.last(o.data));
}

function printFullDefPath(o) {
  let first = _.head(o.data);
  let rest = _.tail(o.data);
  return (
    <>
      <span>{printDefPathData(first)}</span>
      {_.map(rest, (pathData, i) => {
        return <span key={i}>::{printDefPathData(pathData)}</span>;
      })}
    </>
  );
}

function printDefPathData(o) {
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
        return "{{anon_constructor}}"
    } else if ("ImplTrait" in o) {
        return "impl-trait";
    } else if ("ImplTraitAssocTy" in o) {
        return "impl-trait-assoc-ty";
    } else {
        throw new Error("Unknown def path data", o);
    }
}