import { CharRange } from "./CharRange";
import { ExprKind } from "./ExprKind";
import { ObligationIdx } from "./ObligationIdx";

export type Expr = {
  range: CharRange;
  obligations: ObligationIdx[];
  kind: ExprKind;
};
