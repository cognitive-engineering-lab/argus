import { MethodLookupIdx } from "./MethodLookupIdx";

export type ExprKind = "misc" | { type: "methodCall"; data: MethodLookupIdx };
