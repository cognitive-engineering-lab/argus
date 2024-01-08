import { CharRange } from "./CharRange";

export type TraitError = {
  range: CharRange;
  // TODO: predicates aren't typed yet, but should be soon.
  predicate: any;
};
