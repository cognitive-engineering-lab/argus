import type { CharRange } from "./CharRange";
import { Obligation } from "./Obligation";

export type ObligationsInBody = {
  name: string | undefined;
  range: CharRange;
  obligations: Obligation[];
};
