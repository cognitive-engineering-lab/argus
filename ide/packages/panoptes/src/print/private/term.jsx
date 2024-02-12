import React from "react";

import { PrintConst } from "./const";
import { PrintValuePath } from "./path";
import { Angled, CommaSeparated, DBraced } from "./syntax";
import { PrintTy } from "./ty";

export const PrintTerm = ({ o }) => {
  if ("Ty" in o) {
    return <PrintTy o={o.Ty} />;
  } else if ("Const" in o) {
    return <PrintConst o={o.Const} />;
  } else {
    throw new Error("Unknown term", o);
  }
};

export const PrintExpr = ({ o }) => {
  if ("BinOp" in o) {
    const [op, lhs, rhs] = o.BinOp;
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
    return (
      <span>
        <PrintConst o={callable} />(
        {intersperse(args, ", ", (arg, i) => (
          <PrintConst o={arg} key={i} />
        ))}
        )
      </span>
    );
  } else if ("Cast" in o) {
    // TODO: handle cast kinds
    const [castKind, expr, ty] = o.Cast;
    return (
      <Angled>
        <PrintConst o={expr} /> as <PrintTy o={ty} />
      </Angled>
    );
  }
};

const PrintBinOp = ({ o }) => {
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

const PrintUnOp = ({ o }) => {
  if (o === "Not") {
    return "!";
  } else if (o === "Neg") {
    return "-";
  } else {
    throw new Error("Unknown unop", o);
  }
};

export const PrintValueTree = ({ o }) => {
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
          return <PrintAggregateArray o={o} />;
        case "tuple":
          return <PrintAggregateTuple o={o} />;
        case "adtNoVariants":
          return <PrintAggregateAdtNoVariants o={o} />;
        case "adt":
          return (
            <PrintAggregateAdt
              o={o}
              valuePath={o.kind.data}
              kind={o.kind.kind}
            />
          );
        default:
          throw new Error("Unknown aggregate kind", o.kind);
      }
    }
    case "leaf": {
      return <PrintTreeLeaf o={o} />;
    }
  }
};

const PrintAggregateArray = ({ o }) => {
  const components = _.map(o.fields, field => () => <PrintConst o={field} />);
  return (
    <span>
      [<CommaSeparated components={components} />]
    </span>
  );
};

const PrintAggregateTuple = ({ o }) => {
  const components = _.map(o.fields, field => () => <PrintConst o={field} />);
  const trailingComma = o.fields.length === 1 ? "," : "";
  return (
    <span>
      (<CommaSeparated components={components} />
      {trailingComma})
    </span>
  );
};

const PrintAggregateAdtNoVariants = ({ o }) => {
  // TODO: is this right??? We'll want to put the trailing type here
  return "unreachable()";
};

const PrintAggregateAdt = ({ o, valuePath, kind }) => {
  switch (kind.type) {
    case "fn": {
      const head = <PrintValuePath o={valuePath} />;
      const components = _.map(o.fields, field => () => (
        <PrintConst o={field} />
      ));
      return (
        <span>
          {head}(<CommaSeparated components={components} />)
        </span>
      );
    }
    case "const": {
      // TODO: is this right???
      return "";
    }
    case "misc": {
      const components = _.map(
        _.zip(kind.names, o.fields),
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

const PrintTreeLeaf = ({ o }) => {
  const prefix = o.kind.type === "ref" ? "&" : "";
  return (
    <>
      {prefix}
      <PrintConstScalarInt o={o.data} />
    </>
  );
};

export const PrintConstScalarInt = ({ o }) => {
  switch (o.type) {
    case "false":
      return "false";
    case "true":
      return "true";
    case "float": {
      return (
        <>
          {o.data}
          {o.isFinite ? "" : "_"}
        </>
      );
    }

    // NOTE: fallthrough is intentional
    case "int":
    case "char":
    case "misc": {
      return o.data;
    }
    default:
      throw new Error("Unknown scalar int kind", o);
  }
};
