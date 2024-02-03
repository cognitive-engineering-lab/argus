// I was auto-generated, please don't touch me!
export type EvaluationResult = "yes" | "maybe-overflow" | "maybe-ambiguity" | "no";

export interface Obligation { type: "Obligation", predicate: any, hash: ObligationHash, range: CharRange, kind: ObligationKind, necessity: ObligationNecessity, result: EvaluationResult, isSynthetic: boolean, }

export interface ObligationsInBody { name?: Symbol | undefined, range: CharRange, ambiguityErrors: ExprIdx[], traitErrors: ExprIdx[], obligations: Array<Obligation>, exprs: Array<Expr>, methodLookups: Array<MethodLookup>, }

export interface MethodLookup { table: Array<MethodStep>, }

export interface ReceiverAdjStep { ty: any, }

export type ObligationIdx = number;

export type ObligationHash = string;

export interface TreeTopology { children: Record<ProofNodeIdx, Array<ProofNodeIdx>>, parent: Record<ProofNodeIdx, ProofNodeIdx>, }

export type ProofNodeIdx = number;

export interface CharPos { line: number, column: number, }

export interface SerializedTree { root: ProofNodeIdx, nodes: Array<Node>, topology: TreeTopology, errorLeaves: Array<ProofNodeIdx>, unnecessaryRoots: Array<ProofNodeIdx>, }

export type ObligationNecessity = "No" | "ForProfessionals" | "OnError" | "Yes";

export interface MethodStep { recvrTy: ReceiverAdjStep, traitPredicates: Array<ObligationIdx>, }

export type ExprIdx = number;

export type ObligationKind = { type: "success" } | { type: "ambiguous" } | { type: "failure" };

export type ExprKind = { type: "misc" } | { type: "callableExpr" } | { type: "methodReceiver" } | { type: "call" } | { type: "callArg" } | { type: "methodCall", data: MethodLookupIdx, error_recvr: boolean, };

export interface Goal { goal: any, }

export type Node = { type: "result", data: string, } | { type: "candidate", data: Candidate, } | { type: "goal", data: Goal, };

export type Candidate = { type: "impl", data: Impl | undefined, fallback: string, } | { type: "any", data: string, };

export interface FilenameIndex { private_use_as_methods_instead: number, }

export type MethodLookupIdx = number;

export interface CharRange { start: CharPos, end: CharPos, filename: FilenameIndex, }

export interface Expr { range: CharRange, obligations: ObligationIdx[], kind: ExprKind, }
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
