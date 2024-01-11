import { CharRange } from "./CharRange";
import { ObligationHash } from "./ObligationHash";

export type TraitError = {
  range: CharRange;
  candidates: ObligationHash[];
  predicate: any;
};
