import type {
  AdtAggregateKind,
  BinOp,
  Const,
  ExprDef,
  LeafKind,
  Term,
  UnOp,
  ValTree,
  Value
} from "@argus/common/bindings";
import _ from "lodash";
import React from "react";

import {
  Angled,
  CommaSeparated,
  DBraced,
  Parenthesized,
  SqBraced
} from "../syntax";
import { PrintConst } from "./const";
import { PrintConstScalarInt } from "./const";
import { PrintValuePath } from "./path";
import { PrintSymbol, PrintTy } from "./ty";

export const PrintTerm = ({ o }: { o: Term }) => {
  if ("Ty" in o) {
    return <PrintTy o={o.Ty} />;
  }
  if ("Const" in o) {
    return <PrintConst o={o.Const} />;
  }
  throw new Error("Unknown term", o);
};

export const PrintExpr = ({ o }: { o: ExprDef }) => {
  if ("Binop" in o) {
    const [op, lhs, rhs] = o.Binop;
    return op === "Cmp" ? (
      <>
        <PrintConst o={lhs} />
        .cmp
        <Parenthesized>
          <PrintConst o={rhs} />
        </Parenthesized>
      </>
    ) : (
      <>
        <PrintConst o={lhs} />
        <PrintBinOp o={op} />
        <PrintConst o={rhs} />
      </>
    );
  }
  if ("UnOp" in o) {
    const [op, expr] = o.UnOp;
    return (
      <>
        <PrintUnOp o={op} />
        <PrintConst o={expr} />
      </>
    );
  }
  if ("FunctionCall" in o) {
    const [callable, args] = o.FunctionCall;
    const prettyArgs = _.map(args, arg => <PrintConst o={arg} />);
    return (
      <>
        <PrintConst o={callable} />(
        <CommaSeparated components={prettyArgs} />)
      </>
    );
  }
  if ("Cast" in o) {
    // TODO: handle cast kind "use"
    const [_castKind, expr, ty] = o.Cast;
    return (
      <Angled>
        <PrintConst o={expr} /> as <PrintTy o={ty} />
      </Angled>
    );
  }
};

// NOTE: this is the mir BinOp enum so not all operators are "source representable."
// Excluding "Cmp" as it rearranges the operands and doesn't follow the pattern.
const PrintBinOp = ({ o }: { o: Exclude<BinOp, "Cmp"> }) => {
  if (o === "Add" || o === "AddUnchecked" || o === "AddWithOverflow") {
    return "+";
  }
  if (o === "Sub" || o === "SubUnchecked" || o === "SubWithOverflow") {
    return "-";
  }
  if (o === "Mul" || o === "MulUnchecked" || o === "MulWithOverflow") {
    return "*";
  }
  if (o === "Div") {
    return "/";
  }
  if (o === "Rem") {
    return "%";
  }
  if (o === "BitXor") {
    return "^";
  }
  if (o === "BitAnd") {
    return "&";
  }
  if (o === "BitOr") {
    return "|";
  }
  if (o === "Shl" || o === "ShlUnchecked") {
    return "<<";
  }
  if (o === "Shr" || o === "ShrUnchecked") {
    return ">>";
  }
  if (o === "Eq") {
    return "==";
  }
  if (o === "Lt") {
    return "<";
  }
  if (o === "Le") {
    return "<=";
  }
  if (o === "Ne") {
    return "!=";
  }
  if (o === "Ge") {
    return ">=";
  }
  if (o === "Gt") {
    return ">";
  }
  if (o === "Offset") {
    return ".";
  }
  throw new Error("Unknown binop", o);
};

const PrintUnOp = ({ o }: { o: UnOp }) => {
  if (o === "Not") {
    return "!";
  }
  if (o === "Neg") {
    return "-";
  }
  if (o === "PtrMetadata") {
    return "ptr_metadata";
  }
  throw new Error("Unknown unop", o);
};

export const PrintValue = ({ o }: { o: Value }) => (
  <>
    <PrintValueTree o={o.valtree} /> as <PrintTy o={o.ty} />
  </>
);

export const PrintValueTree = ({ o }: { o: ValTree }) => {
  switch (o.type) {
    case "String": {
      const prefix = o.isDeref ? "*" : "";
      return (
        <>
          {prefix}
          {o.data}
        </>
      );
    }
    case "Ref": {
      return (
        <>
          {"&"}
          <PrintValue o={o.inner} />
        </>
      );
    }
    case "Aggregate": {
      switch (o.kind.type) {
        case "Array":
          return <PrintAggregateArray fields={o.fields} />;
        case "Tuple":
          return <PrintAggregateTuple fields={o.fields} />;
        case "AdtNoVariants":
          return <PrintAggregateAdtNoVariants o={o} />;
        case "Adt":
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
    case "Leaf": {
      return <PrintTreeLeaf data={o.data} kind={o.kind} />;
    }
    default:
      throw new Error("Unknown value tree", o);
  }
};

const PrintAggregateArray = ({ fields }: { fields: Const[] }) => {
  const components = _.map(fields, field => <PrintConst o={field} />);
  return (
    <SqBraced>
      <CommaSeparated components={components} />
    </SqBraced>
  );
};

const PrintAggregateTuple = ({ fields }: { fields: Const[] }) => {
  const components = _.map(fields, field => <PrintConst o={field} />);
  const trailingComma = fields.length === 1 ? "," : null;
  return (
    <Parenthesized>
      <CommaSeparated components={components} />
      {trailingComma}
    </Parenthesized>
  );
};

const PrintAggregateAdtNoVariants = ({ o: _ }: { o: unknown }) => {
  // TODO: is this right??? We'll want to put the trailing type here
  return "unreachable()";
};

const PrintAggregateAdt = ({
  fields,
  valuePath,
  kind
}: {
  fields: Const[];
  valuePath: any;
  kind: AdtAggregateKind;
}) => {
  switch (kind.type) {
    case "Fn": {
      const head = <PrintValuePath o={valuePath} />;
      const components = _.map(fields, field => <PrintConst o={field} />);
      return (
        <>
          {head}
          <Parenthesized>
            <CommaSeparated components={components} />
          </Parenthesized>
        </>
      );
    }
    case "Const": {
      // FIXME: seems weird that rustc doesn't print anything here.
      return null;
    }
    case "Misc": {
      const components = _.map(_.zip(kind.names, fields), ([name, field]) => (
        <>
          <PrintSymbol o={name!} />: <PrintConst o={field!} />
        </>
      ));

      return (
        <DBraced>
          <CommaSeparated components={components} />
        </DBraced>
      );
    }
  }
};

const PrintTreeLeaf = ({ data, kind }: { data: any; kind: LeafKind }) => {
  const prefix = kind.type === "Ref" ? "&" : "";
  return (
    <>
      {prefix}
      <PrintConstScalarInt o={data} />
    </>
  );
};
