import { AmbiguityError } from "./AmbiguityError";
import type { CharRange } from "./CharRange";
import { Obligation } from "./Obligation";
import { TraitError } from "./TraitError";

export type ObligationsInBody = {
  name: string | undefined;
  traitErrors: TraitError[];
  ambiguityErrors: AmbiguityError[];
  range: CharRange;
  obligations: Obligation[];
};
