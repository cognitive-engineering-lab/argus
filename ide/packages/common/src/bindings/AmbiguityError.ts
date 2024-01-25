import { CharRange } from "./CharRange";
import { MethodLookup } from "./MethodLookup";

export type AmbiguityError = { range: CharRange; lookup: MethodLookup };
