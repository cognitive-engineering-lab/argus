import type { CharRange } from "./CharRange";
import { Expr } from "./Expr";
import { ExprIdx } from "./ExprIdx";
import { MethodLookup } from "./MethodLookup";
import { Obligation } from "./Obligation";

export type ObligationsInBody = {
  name: string | undefined;
  range: CharRange;
  ambiguityErrors: ExprIdx[];
  traitErrors: ExprIdx[];
  obligations: Obligation[];
  exprs: Expr[];
  methodLookups: MethodLookup[];
};
