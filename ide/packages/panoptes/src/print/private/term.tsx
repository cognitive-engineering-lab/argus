import {
  AdtAggregateKind,
  BinOp,
  Const,
  ExprDef,
  LeafKind,
  Term,
  UnOp,
  ValTree,
} from "@argus/common/bindings";
import _ from "lodash";
import React from "react";

import { PrintConst } from "./const";
import { PrintConstScalarInt } from "./const";
import { PrintValuePath } from "./path";
import { Angled, CommaSeparated, DBraced } from "./syntax";
import { PrintSymbol, PrintTy } from "./ty";

export const PrintTerm = ({ o }: { o: Term }) => {
  if ("Ty" in o) {
    return <PrintTy o={o.Ty} />;
  } else if ("Const" in o) {
    return <PrintConst o={o.Const} />;
  } else {
    throw new Error("Unknown term", o);
  }
};

export const PrintExpr = ({ o }: { o: ExprDef }) => {
  if ("Binop" in o) {
    const [op, lhs, rhs] = o.Binop;
    return (
      <>
        <PrintConst o={lhs} />
        <PrintBinOp o={op} />
        <PrintConst o={rhs} />
      </>
    );
  } else if ("UnOp" in o) {
    const [op, expr] = o.UnOp;
    return (
      <>
        <PrintUnOp o={op} />
        <PrintConst o={expr} />
      </>
    );
  } else if ("FunctionCall" in o) {
    const [callable, args] = o.FunctionCall;
    const argEs = _.map(args, arg => () => <PrintConst o={arg} />);
    return (
      <span>
        <PrintConst o={callable} />(
        <CommaSeparated components={argEs} />)
      </span>
    );
  } else if ("Cast" in o) {
    // TODO: handle cast kind "use"
    const [castKind, expr, ty] = o.Cast;
    return (
      <Angled>
        <PrintConst o={expr} /> as <PrintTy o={ty} />
      </Angled>
    );
  }
};

const PrintBinOp = ({ o }: { o: BinOp }) => {
  if (o === "Add") {
    return "+";
  } else if (o === "AddUnchecked") {
    return "+";
  } else if (o === "Sub") {
    return "-";
  } else if (o === "SubUnchecked") {
    return "-";
  } else if (o === "Mul") {
    return "*";
  } else if (o === "MulUnchecked") {
    return "*";
  } else if (o === "Div") {
    return "/";
  } else if (o === "Rem") {
    return "%";
  } else if (o === "BitXor") {
    return "^";
  } else if (o === "BitAnd") {
    return "&";
  } else if (o === "BitOr") {
    return "|";
  } else if (o === "Shl") {
    return "<<";
  } else if (o === "ShlUnchecked") {
    return "<<";
  } else if (o === "Shr") {
    return ">>";
  } else if (o === "ShrUnchecked") {
    return ">>";
  } else if (o === "Eq") {
    return "==";
  } else if (o === "Lt") {
    return "<";
  } else if (o === "Le") {
    return "<=";
  } else if (o === "Ne") {
    return "!=";
  } else if (o === "Ge") {
    return ">=";
  } else if (o === "Gt") {
    return ">";
  } else if (o === "Offset") {
    return ".";
  } else {
    throw new Error("Unknown binop", o);
  }
};

const PrintUnOp = ({ o }: { o: UnOp }) => {
  if (o === "Not") {
    return "!";
  } else if (o === "Neg") {
    return "-";
  } else {
    throw new Error("Unknown unop", o);
  }
};

export const PrintValueTree = ({ o }: { o: ValTree }) => {
  switch (o.type) {
    case "string": {
      // TODO: do we need to escape something here?
      const prefix = o.isDeref ? "*" : "";
      return (
        <>
          {prefix}
          {o.data}
        </>
      );
    }
    case "ref": {
      return (
        <>
          {"&"}
          <PrintValueTree o={o.inner} />
        </>
      );
    }
    case "aggregate": {
      switch (o.kind.type) {
        case "array":
          return <PrintAggregateArray fields={o.fields} />;
        case "tuple":
          return <PrintAggregateTuple fields={o.fields} />;
        case "adtnovariants":
          return <PrintAggregateAdtNoVariants o={o} />;
        case "adt":
          return (
            <PrintAggregateAdt
              fields={o.fields}
              valuePath={o.kind.data}
              kind={o.kind.kind}
            />
          );
        default:
          throw new Error("Unknown aggregate kind", o.kind);
      }
    }
    case "leaf": {
      return <PrintTreeLeaf data={o.data} kind={o.kind} />;
    }
  }
};

const PrintAggregateArray = ({ fields }: { fields: Const[] }) => {
  const components = _.map(fields, field => () => <PrintConst o={field} />);
  return (
    <span>
      [<CommaSeparated components={components} />]
    </span>
  );
};

const PrintAggregateTuple = ({ fields }: { fields: Const[] }) => {
  const components = _.map(fields, field => () => <PrintConst o={field} />);
  const trailingComma = fields.length === 1 ? "," : null;
  return (
    <>
      (<CommaSeparated components={components} />
      {trailingComma})
    </>
  );
};

const PrintAggregateAdtNoVariants = ({ o: _ }: { o: unknown }) => {
  // TODO: is this right??? We'll want to put the trailing type here
  return "unreachable()";
};

const PrintAggregateAdt = ({
  fields,
  valuePath,
  kind,
}: {
  fields: Const[];
  valuePath: any;
  kind: AdtAggregateKind;
}) => {
  switch (kind.type) {
    case "fn": {
      const head = <PrintValuePath o={valuePath} />;
      const components = _.map(fields, field => () => <PrintConst o={field} />);
      return (
        <>
          {head}(<CommaSeparated components={components} />)
        </>
      );
    }
    case "const": {
      // TODO: is this right???
      return "";
    }
    case "misc": {
      const components = _.map(
        _.zip(kind.names, fields),
        ([name, field]) =>
          () =>
            (
              <span>
                <PrintSymbol o={name} />: <PrintConst o={field} />
              </span>
            )
      );

      return (
        <DBraced>
          <CommaSeparated components={components} />
        </DBraced>
      );
    }
  }
};

const PrintTreeLeaf = ({ data, kind }: { data: any; kind: LeafKind }) => {
  const prefix = kind.type === "ref" ? "&" : "";
  return (
    <>
      {prefix}
      <PrintConstScalarInt o={data} />
    </>
  );
};
