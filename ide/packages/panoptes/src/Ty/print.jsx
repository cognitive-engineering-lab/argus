
function printObligation(o) {
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
    // TODO use the polarity
    let polarity = o.polarity === "Negative" ? "!" : "";
    return (
        <span>{polarity}</span><span>{printTraitRef(o.trait_ref);}</span>
    );
}

function printTraitRef(o) {
    return (
        <span>
            <span>{printTy(o.self_ty)}</span>
            as
            <span>{printTraitPath(o.trait_path)}</span>
        </span>
    );
}

function printTraitPath(o) {
    throw new Error("TODO");
}