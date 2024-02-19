import {
  Const,
  ConstScalarInt,
  InferConst,
  ParamConst,
  UnevaluatedConst,
} from "@argus/common/bindings";
import React from "react";

import { PrintDefPath, PrintValuePath } from "./path";
import { PrintExpr, PrintValueTree } from "./term";
import { PrintBoundVariable, PrintSymbol } from "./ty";

export const PrintConst = ({ o }: { o: Const }) => {
  console.debug("Printing const", o);
  switch (o.type) {
    case "Error":
      return "{{const error}}";
    case "Param":
      return <PrintParamConst o={o.data} />;
    case "Infer":
      return <PrintInferConst o={o.data} />;
    case "Bound":
      return <PrintBoundVariable o={o.data} />;
    // case "placeholder":
    //   throw new Error("TODO");
    case "Unevaluated":
      return <PrintUnevaluatedConst o={o.data} />;
    case "Value":
      return <PrintValueTree o={o.data} />;
    case "Expr":
      return <PrintExpr o={o.data} />;
    default:
      throw new Error("Unknown const kind", o);
  }
};

const PrintInferConst = ({ o }: { o: InferConst }) => {
  switch (o.type) {
    case "Anon": {
      return <span>_</span>;
    }
    default:
      throw new Error("Unknown infer const kind", o as any);
  }
};

const PrintParamConst = ({ o }: { o: ParamConst }) => {
  return <PrintSymbol o={o} />;
};

const PrintUnevaluatedConst = ({ o }: { o: UnevaluatedConst }) => {
  switch (o.type) {
    case "ValuePath": {
      return <PrintValuePath o={o.data} />;
    }
    case "AnonSnippet": {
      return o.data;
    }
    case "AnonLocation": {
      return (
        <>
          <PrintSymbol o={o.krate} />
          ::
          <PrintDefPath o={o.path} />
        </>
      );
    }
    default:
      throw new Error("Unknown unevaluated const kind", o);
  }
};

export const PrintConstScalarInt = ({ o }: { o: ConstScalarInt }) => {
  switch (o.type) {
    case "False":
      return "false";
    case "True":
      return "true";
    case "Float": {
      return (
        <>
          {o.data}
          {o.isFinite ? "" : "_"}
        </>
      );
    }

    // NOTE: fallthrough is intentional
    case "Int":
    case "Char":
    case "Misc": {
      return o.data;
    }
    default:
      throw new Error("Unknown scalar int kind", o);
  }
};
