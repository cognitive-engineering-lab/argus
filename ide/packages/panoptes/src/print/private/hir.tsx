import {
  AnonConst,
  GenericArgs,
  GenericBound,
  GenericParam,
  Generics,
  Ident,
  Impl,
  Lifetime,
  MutTy,
  Mutability,
  ParamName,
  Path,
  PathSegment,
  PolyTraitRef,
  QPath,
  Symbol,
  Term,
  TraitRef,
  Ty,
  TyKind,
  TypeBinding,
} from "@argus/common/bindings";
import _ from "lodash";
import React from "react";

import { HoverInfo } from "../../HoverInfo";
import "./hir.css";
import * as kw from "./syntax";

function isObject(x: any): x is object {
  return typeof x === "object" && x !== null;
}

const genericArgsNone: GenericArgs = {
  args: [],
  bindings: [],
  parenthesized: "No",
};

const CommaSeparated = ({ children }: { children: React.ReactNode[] }) => {
  return (
    <span>
      {_.map(children, (child, i) => (
        <span key={i}>
          {i > 0 ? ", " : ""}
          {child}
        </span>
      ))}
    </span>
  );
};

const Angled = ({ child }: { child: React.ReactNode }) => {
  const [lt, gt] = ["<", ">"];
  return (
    <span>
      {lt}
      {child}
      {gt}
    </span>
  );
};

function paramName(param: ParamName): Ident {
  return param === "Error" || param === "Fresh"
    ? kw.UnderscoreLifetime
    : param.Plain;
}

export const PrintImpl = ({ impl }: { impl: Impl }) => {
  console.debug("Printing Impl", impl);

  const generics =
    impl.generics.params.length === 0 ? (
      ""
    ) : (
      <span>
        <PrintGenericsParams generics={impl.generics} />{" "}
      </span>
    );

  const polarity = impl.polarity === "Positive" ? "" : "!";

  const ofTrait =
    impl.of_trait !== undefined ? (
      <span>
        <PrintTraitRef traitRef={impl.of_trait} />{" "}
        <span className="kw">for</span>{" "}
      </span>
    ) : (
      ""
    );

  const ty = <PrintTy ty={impl.self_ty} />;
  const whereClause = <PrintWhereClause generics={impl.generics} />;

  // TODO: the where clauses need to go in a tooltip
  return (
    <span>
      <span className="kw">impl</span>
      {generics}
      {polarity}
      {ofTrait}
      {ty}
      {whereClause}
    </span>
  );
};

const PrintWhereClause = ({ generics }: { generics: Generics }) => {
  if (generics.predicates.length === 0) {
    return "";
  }

  const whereHoverContent = () => (
    <div className="DirNode WhereConstraintArea">
      {_.map(generics.predicates, (pred, idx) => {
        const innerContent =
          "BoundPredicate" in pred ? (
            <span>
              <PrintFormalGenericParams
                params={pred.BoundPredicate.bound_generic_params}
              />
              <PrintTy ty={pred.BoundPredicate.bounded_ty} />
              <PrintBounds prefix=":" bounds={pred.BoundPredicate.bounds} />
            </span>
          ) : "RegionPredicate" in pred ? (
            <span>
              <PrintLifetime lifetime={pred.RegionPredicate.lifetime} />:
              {_.map(pred.RegionPredicate.bounds, (bound, idx) =>
                "Outlives" in bound ? (
                  <PrintLifetime lifetime={bound.Outlives} key={idx} />
                ) : (
                  "ERROR UNKNOWN"
                )
              )}
            </span>
          ) : "EqPredicate" in pred ? (
            <span>
              <PrintTy ty={pred.EqPredicate.lhs_ty} /> ={" "}
              <PrintTy ty={pred.EqPredicate.rhs_ty} />
            </span>
          ) : (
            ""
          );

        return (
          <div className="WhereConstraint" key={idx}>
            {innerContent}
          </div>
        );
      })}
    </div>
  );

  return (
    <span>
      {" "}
      <span className="kw">where</span>{" "}
      <HoverInfo Content={whereHoverContent}>
        <span className="where">...</span>
      </HoverInfo>
    </span>
  );
};

const PrintGenericsParams = ({ generics }: { generics: Generics }) => {
  const params = generics.params;
  if (params.length == 0) {
    return "";
  }

  const innerElems = _.map(params, (param, idx) => (
    <PrintGenericParam param={param} key={idx} />
  ));

  return <Angled child={<CommaSeparated children={innerElems} />} />;
};

const PrintGenericParam = ({ param }: { param: GenericParam }) => {
  const prefix = "Const" in param.kind ? "const " : "";
  const ident = <PrintIdent ident={paramName(param.name)} />;
  const after =
    "Lifetime" in param.kind ? (
      ""
    ) : "Type" in param.kind && param.kind.Type.default !== undefined ? (
      <span>
        {" = "}
        {<PrintTy ty={param.kind.Type.default} />}
      </span>
    ) : "Const" in param.kind ? (
      <span>
        {": "}
        <PrintTy ty={param.kind.Const.ty} />
        {param.kind.Const.default !== undefined ? (
          <span>
            {" = "}
            <PrintAnonConst anon={param.kind.Const.default} />
          </span>
        ) : (
          ""
        )}
      </span>
    ) : (
      ""
    );

  return (
    <span>
      {prefix}
      {ident}
      {after}
    </span>
  );
};

const PrintAnonConst = ({ anon }: { anon: AnonConst }) => {
  return "TODO: anonconst";
};

const PrintIdent = ({ ident }: { ident: Ident }) => {
  return <span>{ident.name}</span>;
};

const PrintTraitRef = ({ traitRef }: { traitRef: TraitRef }) => {
  return <PrintPath path={traitRef.path} />;
};

const PrintPath = ({
  path,
  colonsBefore = false,
}: {
  path: Path;
  colonsBefore?: boolean;
}) => {
  return (
    <span>
      {_.map(path.segments, (segment, idx) => (
        <span>
          {idx > 0 ? "::" : ""}
          {segment.ident !== kw.PathRoot ? (
            <>
              <PrintIdent ident={segment.ident} />
              <PrintGenericArgs
                args={segment.args}
                colonBefore={colonsBefore}
              />
            </>
          ) : (
            ""
          )}
        </span>
      ))}
    </span>
  );
};

function genericArgsInputs(args: GenericArgs): Ty[] | undefined {
  if (args.parenthesized !== "ParenSugar") {
    return;
  }

  for (let arg of args.args) {
    if ("Type" in arg && isObject(arg.Type.kind) && "Tup" in arg.Type.kind) {
      return arg.Type.kind.Tup;
    }
  }
}

function genericArgsReturn(args: GenericArgs): Ty | undefined {
  const bk = _.first(args.bindings)?.kind;
  if (bk !== undefined && "Equality" in bk) {
    if ("Ty" in bk.Equality.term) {
      return bk.Equality.term.Ty;
    }
  }
}

const PrintGenericArgs = ({
  args,
  colonBefore,
}: {
  args: GenericArgs | undefined;
  colonBefore: boolean;
}) => {
  const uArgs = args ?? genericArgsNone;

  switch (uArgs.parenthesized) {
    case "No": {
      // SEE: https://github.com/rust-lang/rust/blob/0ea334ab739265168fba366afcdc7ff68c1dec53/compiler/rustc_hir_pretty/src/lib.rs#L1620
      const start = colonBefore ? "::<" : "<";
      let empty = true;
      // TODO: wtf is this????
      const startOrComma = () => {
        if (empty) {
          empty = false;
          return start;
        } else {
          return ", ";
        }
      };

      // SEE https://github.com/rust-lang/rust/blob/0ea334ab739265168fba366afcdc7ff68c1dec53/compiler/rustc_hir_pretty/src/lib.rs#L1632-L1643
      // for when we want to make eliding arguments possible.
      let nonelidedGenericArgs = uArgs.args.length > 0;

      let nonElided = !nonelidedGenericArgs
        ? ""
        : (() => {
            const prefix = startOrComma();
            const commsep = (
              <CommaSeparated
                children={_.map(uArgs.args, (genA, idx) => {
                  if ("Lifetime" in genA) {
                    return <PrintLifetime lifetime={genA.Lifetime} key={idx} />;
                  } else if ("Type" in genA) {
                    return <PrintTy ty={genA.Type} key={idx} />;
                  } else if ("Const" in genA) {
                    return <PrintAnonConst anon={genA.Const.value} key={idx} />;
                  } else if ("Infer" in genA) {
                    return "_";
                  }
                })}
              />
            );
            return (
              <span>
                {prefix}
                {commsep}
              </span>
            );
          })();

      const bindings = _.map(uArgs.bindings, (binding, idx) => (
        <span>
          {startOrComma()}
          <PrintTypeBinding binding={binding} key={idx} />
        </span>
      ));

      const end = empty ? "" : ">";

      return (
        <span>
          {nonElided}
          {bindings}
          {end}
        </span>
      );
    }
    case "ParenSugar": {
      const inputs = genericArgsInputs(uArgs);
      const argList = (
        <span>
          (
          <CommaSeparated
            children={_.map(inputs, (a, i) => (
              <PrintTy ty={a} key={i} />
            ))}
          />
          )
        </span>
      );
      const arr = " -> ";
      const ret = <PrintTy ty={genericArgsReturn(uArgs)!} />;

      return (
        <span>
          {argList}
          {arr}
          {ret}
        </span>
      );
    }
    case "ReturnTypeNotation": {
      return "(..)";
    }
  }
};

const PrintTerm = ({ term }: { term: Term }) => {
  if ("Ty" in term) {
    return <PrintTy ty={term.Ty} />;
  } else if ("Const" in term) {
    return <PrintAnonConst anon={term.Const} />;
  }
};

const PrintBounds = ({
  prefix,
  bounds,
}: {
  prefix: string;
  bounds: GenericBound[];
}) => {
  return (
    <span>
      {_.map(bounds, (bound, idx) => {
        const prfx = idx == 0 ? (prefix.length > 0 ? prefix + " " : "") : "+ ";
        if ("Trait" in bound) {
          const mb = bound.Trait[1] === "Maybe" ? "?" : "";
          return (
            <span key={idx}>
              {prfx}
              {mb}
              <PrintPolyTraitRef pTraitRef={bound.Trait[0]} />
            </span>
          );
        } else if ("Outlives" in bound) {
          return (
            <span key={idx}>
              {prfx}
              <PrintLifetime lifetime={bound.Outlives} />
            </span>
          );
        }
      })}
    </span>
  );
};

const PrintPolyTraitRef = ({ pTraitRef }: { pTraitRef: PolyTraitRef }) => {
  return (
    <>
      <PrintFormalGenericParams params={pTraitRef.bound_generic_params} />
      <PrintTraitRef traitRef={pTraitRef.trait_ref} />
    </>
  );
};

const PrintFormalGenericParams = ({ params }: { params: GenericParam[] }) => {
  if (params.length === 0) {
    return "";
  }

  return (
    <span>
      for
      <PrintGenericParams params={params} />
    </span>
  );
};

const PrintGenericParams = ({ params }: { params: GenericParam[] }) => {
  if (params.length == 0) {
    return "";
  }
  const inner = (
    <CommaSeparated
      children={_.map(params, (p, i) => (
        <PrintGenericParam param={p} key={i} />
      ))}
    />
  );
  return <Angled child={inner} />;
};

const PrintTypeBinding = ({ binding }: { binding: TypeBinding }) => {
  const id = <PrintIdent ident={binding.ident} />;
  const genArgs = (
    <PrintGenericArgs args={binding.gen_args} colonBefore={false} />
  );
  const rest =
    "Equality" in binding.kind ? (
      <span>
        {"= "}
        <PrintTerm term={binding.kind.Equality.term} />
      </span>
    ) : "Constraint" in binding.kind ? (
      <PrintBounds prefix={":"} bounds={binding.kind.Constraint.bounds} />
    ) : (
      ""
    );
  return (
    <span>
      {id}
      {genArgs} {rest}
    </span>
  );
};

const PrintTy = ({ ty }: { ty: Ty }) => {
  return <PrintTyKind tyKind={ty.kind} />;
};

const PrintTyKind = ({ tyKind }: { tyKind: TyKind }) => {
  if (tyKind === "Never") {
    return "!";
  } else if (tyKind === "Infer") {
    return "_";
  } else if (tyKind === "Err") {
    return "/*ERROR*/";
  } else if ("InferDelegation" in tyKind) {
    return "_";
  } else if ("Slice" in tyKind) {
    return (
      <span>
        [<PrintTy ty={tyKind.Slice} />]
      </span>
    );
  } else if ("Ptr" in tyKind) {
    return (
      <span>
        *<PrintMutTy mty={tyKind.Ptr} />
      </span>
    );
  } else if ("Ref" in tyKind) {
    const [lifetime, ty] = tyKind.Ref;
    return (
      <span>
        &<PrintLifetime lifetime={lifetime} />
        <PrintMutTy mty={ty} />
      </span>
    );
  } else if ("Tup" in tyKind) {
    return (
      <CommaSeparated
        children={_.map(tyKind.Tup, (ty, i) => (
          <PrintTy ty={ty} key={i} />
        ))}
      />
    );
  } else if ("BareFn" in tyKind) {
    return "TODO: BAREFN";
  } else if ("OpaqueDef" in tyKind) {
    return "TODO: OPAQUEDEF";
  } else if ("Path" in tyKind) {
    return <PrintQPath qpath={tyKind.Path} colonsBefore={false} />;
  } else if ("TraitObject" in tyKind) {
    return "TODO: TRAITOBJECT";
  } else if ("Array" in tyKind) {
    return "TODO: ARRAY";
  } else if ("Typeof" in tyKind) {
    return (
      <span>
        typeof <PrintAnonConst anon={tyKind.Typeof} />
      </span>
    );
  }
};

const PrintMutTy = ({
  mty,
  printConst = true,
}: {
  mty: MutTy;
  printConst?: boolean;
}) => {
  return (
    <>
      <PrintMutability mtbl={mty.mutbl} printConst={printConst} />
      <PrintTy ty={mty.ty} />
    </>
  );
};

const PrintMutability = ({
  mtbl,
  printConst,
}: {
  mtbl: Mutability;
  printConst: boolean;
}) => {
  return mtbl === "Mut" ? "mut " : printConst ? "const " : "";
};

const PrintLifetime = ({ lifetime }: { lifetime: Lifetime }) => {
  return <PrintIdent ident={lifetime.ident} />;
};

const PrintQPath = ({
  qpath,
  colonsBefore,
}: {
  qpath: QPath;
  colonsBefore: boolean;
}) => {
  if ("LangItem" === qpath) {
    return "#[lang = (...)]";
  } else if ("Resolved" in qpath && qpath.Resolved[0] === null) {
    return <PrintPath path={qpath.Resolved[1]} />;
  } else if ("Resolved" in qpath && qpath.Resolved[0] !== null) {
    const [ty, path] = qpath.Resolved;
    const inner = (
      <span>
        <PrintTy ty={ty} />
        {" as "}
      </span>
    );
    const listed = _.map(path.segments.slice(-1), (seg, idx) => {
      const prefix = idx > 0 ? "::" : "";
      return (
        <span>
          {prefix}
          <PrintPathSegment
            segment={seg}
            colonsBefore={colonsBefore}
            key={idx}
          />
        </span>
      );
    });
    const angles = (
      <Angled
        child={
          <span>
            {inner}
            {listed}
          </span>
        }
      />
    );
    const lastSegment = (
      <PrintPathSegment segment={path.segments[path.segments.length - 1]} />
    );
    return (
      <span>
        {angles}::{lastSegment}
      </span>
    );
  } else if ("TypeRelative" in qpath) {
    const [ty, segment] = qpath.TypeRelative;
    // FIXME: woof ...
    const prefix =
      isObject(ty.kind) &&
      "Path" in ty.kind &&
      isObject(ty.kind.Path) &&
      "Resolved" in ty.kind.Path &&
      ty.kind.Path.Resolved[0] === undefined ? (
        <PrintTy ty={ty} />
      ) : (
        <Angled child={<PrintTy ty={ty} />} />
      );
    return (
      <span>
        {prefix}::
        <PrintPathSegment segment={segment} colonsBefore={colonsBefore} />
      </span>
    );
  }
};

const PrintPathSegment = ({
  segment,
  colonsBefore = false,
}: {
  segment: PathSegment;
  colonsBefore?: boolean;
}) => {
  if (segment.ident === kw.PathRoot) {
    return "";
  }

  return (
    <>
      <PrintIdent ident={segment.ident} />
      <PrintGenericArgs args={segment.args} colonBefore={colonsBefore} />
    </>
  );
};
