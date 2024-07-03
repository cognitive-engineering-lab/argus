// NOTE: this file needs to stay up to date with
// the serializeid hir types, but don't remove me!
export type Symbol = string;

export type Impl = {
  polarity: ImplPolarity;
  generics: Generics;
  of_trait: TraitRef | undefined;
  self_ty: Ty;
};

export type ImplPolarity = "Positive" | "Negative";

export type Generics = {
  params: GenericParam[];
  predicates: WherePredicate[];
};

export type TraitRef = {
  path: Path;
};

export type PolyTraitRef = {
  bound_generic_params: GenericParam[];
  trait_ref: TraitRef;
};

export type Ty = {
  kind: TyKind;
};

export type TyKind =
  | { InferDelegation: {} }
  | { Slice: Ty }
  | { Array: Ty }
  | { Ptr: MutTy }
  | { Ref: [Lifetime, MutTy] }
  | { BareFn: {} }
  | "Never"
  | { Tup: Ty[] }
  | { Path: QPath }
  | { OpaqueDef: {} }
  | { TraitObject: [PolyTraitRef[], Lifetime] }
  | { Typeof: {} }
  | "Infer"
  | "Err";

export type MutTy = {
  ty: Ty;
  mutbl: Mutability;
};

export type Mutability = "Mut" | "Not";

export type QPath =
  | { Resolved: [Ty | null, Path] }
  | { TypeRelative: [Ty, PathSegment] }
  | "LangItem";

export type GenericParam = {
  name: ParamName;
  kind: GenericParamKind;
};

export type GenericParamKind =
  | { Lifetime: Lifetime }
  | { Type: { default: Ty | undefined } }
  | { Const: { ty: Ty; default: AnonConst | undefined } };

export type ParamName = { Plain: Ident } | "Fresh" | "Error";

export type WherePredicate =
  | { BoundPredicate: WhereBoundPredicate }
  | { RegionPredicate: WhereRegionPredicate }
  | { EqPredicate: WhereEqPredicate };

export type WhereBoundPredicate = {
  bound_generic_params: GenericParam[];
  bounded_ty: Ty;
  bounds: GenericBound[];
};

export type WhereRegionPredicate = {
  lifetime: Lifetime;
  bounds: GenericBound[];
};

export type WhereEqPredicate = {
  lhs_ty: Ty;
  rhs_ty: Ty;
};

export type Lifetime = {
  ident: Ident;
};

export type Ident = {
  name: Symbol;
};

export type Path = {
  segments: PathSegment[];
};

export type PathSegment = {
  ident: Ident;
  args: GenericArgs | undefined;
};

export type GenericArgs = {
  args: GenericArg[];
  bindings: TypeBinding[];
  parenthesized: GenericArgsParentheses;
};

export type GenericArg =
  | { Lifetime: Lifetime }
  | { Type: Ty }
  | { Const: ConstArg }
  | { Infer: InferArg };

export type AnonConst = {};

export type ConstArg = {
  value: AnonConst;
};

export type InferArg = {};

export type GenericArgsParentheses = "No" | "ReturnTypeNotation" | "ParenSugar";

export type TypeBinding = {
  ident: Ident;
  gen_args: GenericArgs;
  kind: TypeBindingKind;
};

export type TypeBindingKind =
  | { Constraint: { bounds: GenericBound[] } }
  | { Equality: { term: Term } };

export type GenericBound =
  | { Trait: [PolyTraitRef, TraitBoundModifier] }
  | { Outlives: Lifetime };

export type TraitBoundModifier =
  | "None"
  | "Negative"
  | "Maybe"
  | "MaybeConst"
  | "Const";

export type Term = { Ty: Ty } | { Const: AnonConst };
