import { MethodStep } from "./MethodStep";
import { ObligationHash } from "./ObligationHash";

export type MethodLookup = {
  table: MethodStep[];
  unmarked: ObligationHash[];
};
