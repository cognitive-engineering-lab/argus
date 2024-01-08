import type { CharRange } from "./CharRange";
import { Obligation } from "./Obligation";
import { TraitError } from "./TraitError";

export type ObligationsInBody = {
  name: string | undefined;
  traitErrors: TraitError[];
  range: CharRange;
  obligations: Obligation[];
};
