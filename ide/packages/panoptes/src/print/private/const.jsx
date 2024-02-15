import React from "react";

import { PrintValuePath } from "./path";
import { PrintExpr, PrintValueTree } from "./term";
import { PrintBoundVariable, PrintSymbol } from "./ty";

export const PrintConst = ({ o }) => {
  console.debug("Printing const", o);
  switch (o.type) {
    case "error":
      return "{{const error}}";
    case "param":
      return <PrintParamConst o={o.data} />;
    case "infer":
      return <PrintInferConst o={o.data} />;
    case "bound":
      return <PrintBoundVariable o={o.data} />;
    case "Placeholder":
      throw new Error("TODO");
    case "unevaluated":
      return <PrintUnevaluatedConst o={o.data} />;
    case "value":
      return <PrintValueTree o={o.data} />;
    case "expr":
      return <PrintExpr o={o.data} />;
    default:
      throw new Error("Unknown const kind", o);
  }
};

const PrintInferConst = ({ o }) => {
  switch (o.type) {
    case "anon": {
      return <span>_</span>;
    }
    default:
      throw new Error("Unknown infer const kind", o);
  }
};

const PrintParamConst = ({ o }) => {
  return <PrintSymbol o={o.name} />;
};

const PrintUnevaluatedConst = ({ o }) => {
  switch (o.type) {
    case "valuePath": {
      return <PrintValuePath o={o.data} />;
    }
    case "anonSnippet": {
      return o.data;
    }
    case "nonLocalPath": {
      throw new Error("todo");
    }
    default:
      throw new Error("Unknown unevaluated const kind", o);
  }
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
