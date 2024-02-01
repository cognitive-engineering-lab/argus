import { Impl } from "./serialization/hir/types";

export type Candidate =
  | { type: "any"; data: string }
  // Impl { ty: TyDef; traitRef: TraitRefDef }
  | { type: "impl"; data: Impl | undefined; fallback: string };
