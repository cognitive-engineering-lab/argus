export type Candidate =
  | { type: "any"; data: string }
  // Impl { ty: TyDef; traitRef: TraitRefDef }
  | { type: "impl"; ty: any; traitRef: any };
