//! Serializing for HIR items, this is used for serializing impl blocks.

use rustc_hir::{def::{Res, NonMacroAttrKind, DefKind}, def_id::{DefId, LocalDefId}, *};
use rustc_span::{symbol::{Ident, Symbol}, ErrorGuaranteed, Span};
use rustc_ast::ast::TraitObjectSyntax;

use serde::{Serialize, Serializer};
use super::{ty::SymbolDef, serialize_custom_seq};


#[derive(Serialize)]
struct NoOp(#[serde(skip_serializing_if = "Option::is_none")] Option<()>);
const NOOP: NoOp = NoOp(None);

#[derive(Serialize)]
#[serde(remote = "Ident")]
pub struct IdentDef {
    #[serde(with = "SymbolDef")]
    pub name: Symbol,

    #[serde(skip)]
    pub span: Span,
}

#[derive(Serialize)]
#[serde(remote = "PathSegment")]
pub struct PathSegmentDef<'hir> {
  #[serde(with = "IdentDef")]
  pub ident: Ident,

  #[serde(skip)]
  pub hir_id: HirId,

  #[serde(with = "ResDef")]
  pub res: Res,

  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(with = "Option__GenericArgsDef")]
  pub args: Option<&'hir GenericArgs<'hir>>,

  #[serde(skip)]
  pub infer_args: bool,
}

pub struct Slice__PathSegmentDef;
impl Slice__PathSegmentDef {
    fn serialize<'hir, S>(value: &[PathSegment<'hir>], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'a>(
            #[serde(with = "PathSegmentDef")]
            &'a PathSegment<'a>,
        );
        serialize_custom_seq! { Wrapper, s, value}
    }
}

pub struct Option__GenericArgsDef;
impl Option__GenericArgsDef {
    fn serialize<'hir, S>(value: &Option<&'hir GenericArgs<'hir>>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            None => NOOP.serialize(s),
            Some(ga) =>GenericArgsDef::serialize(ga, s),
        }
    }
}

#[derive(Serialize)]
#[serde(remote = "GenericArgs")]
pub struct GenericArgsDef<'hir> {
  #[serde(with = "Slice__GenericArgDef")]
  pub args: &'hir [GenericArg<'hir>],

  #[serde(with = "Slice__TypeBindingDef")]
  pub bindings: &'hir [TypeBinding<'hir>],

  #[serde(with = "GenericArgsParenthesesDef")]
  pub parenthesized: GenericArgsParentheses,

  #[serde(skip)]
  pub span_ext: Span,
}

pub struct Slice__GenericArgDef;
impl Slice__GenericArgDef {
    fn serialize<'hir, S>(value: &[GenericArg<'hir>], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'a>(
            #[serde(with = "GenericArgDef")]
            &'a GenericArg<'a>,
        );
        serialize_custom_seq! { Wrapper, s, value}
    }
}

#[derive(Serialize)]
#[serde(remote = "GenericArgsParentheses")]
pub enum GenericArgsParenthesesDef {
  No,
  ReturnTypeNotation,
  ParenSugar,
}

#[derive(Serialize)]
#[serde(remote = "GenericArg")]
pub enum GenericArgDef<'hir> {
  Lifetime(#[serde(with = "LifetimeDef")] &'hir Lifetime),
  Type(#[serde(with = "TyDef")] &'hir Ty<'hir>),
  Const(#[serde(with = "ConstArgDef")] ConstArg),
  Infer(#[serde(with = "InferArgDef")] InferArg),
}

#[derive(Serialize)]
#[serde(remote = "Lifetime")]
pub struct LifetimeDef {
  #[serde(skip)]
  pub hir_id: HirId,

  #[serde(with = "IdentDef")]
  pub ident: Ident,

  #[serde(skip)]
  pub res: LifetimeName,
}

#[derive(Serialize)]
#[serde(remote = "ConstArg")]
pub struct ConstArgDef {
  #[serde(with = "AnonConstDef")]
  pub value: AnonConst,

  #[serde(skip)]
  pub span: Span,

  #[serde(skip)]
  pub is_desugared_from_effects: bool,
}

#[derive(Serialize)]
#[serde(remote = "InferArg")]
pub struct InferArgDef {
  #[serde(skip)]
  pub hir_id: HirId,

  #[serde(skip)]
  pub span: Span,
}

#[derive(Serialize)]
#[serde(remote = "Ty")]
pub struct TyDef<'hir> {
  #[serde(skip)]
  pub hir_id: HirId,

  #[serde(with = "TyKindDef")]
  pub kind: TyKind<'hir>,

  #[serde(skip)]
  pub span: Span,
}

pub struct Slice__TyDef;
impl Slice__TyDef {
    fn serialize<'hir, S>(value: &[Ty<'hir>], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'a>(
            #[serde(with = "TyDef")]
            &'a Ty<'a>,
        );
        serialize_custom_seq! { Wrapper, s, value}
    }
}

pub struct Option__TyDef;
impl Option__TyDef {
    fn serialize<'hir, S>(value: &Option<&Ty<'hir>>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            None => NOOP.serialize(s),
            Some(ga) => TyDef::serialize(ga, s),
        }
    }
}

#[derive(Serialize)]
#[serde(remote = "MutTy")]
pub struct MutTyDef<'hir> {
    #[serde(with = "TyDef")]
    pub ty: &'hir Ty<'hir>,

    #[serde(with = "MutabilityDef")]
    pub mutbl: Mutability,
}

#[derive(Serialize)]
#[serde(remote = "Mutability")]
pub enum MutabilityDef {
    Not,
    Mut,
}

#[derive(Serialize)]
#[serde(remote = "TyKind")]
pub enum TyKindDef<'hir> {
  InferDelegation(
      #[serde(skip)]
      DefId,

      #[serde(skip)]
      InferDelegationKind
  ),

  Slice(
      #[serde(with = "TyDef")]
      &'hir Ty<'hir>
  ),

  Array(
      #[serde(with = "TyDef")]
      &'hir Ty<'hir>,

      #[serde(skip)]
      ArrayLen
  ),

  Ptr(
      #[serde(with = "MutTyDef")]
      MutTy<'hir>
  ),

  Ref(
      #[serde(with = "LifetimeDef")]
      &'hir Lifetime,

      #[serde(with = "MutTyDef")]
      MutTy<'hir>
  ),

  BareFn(
      #[serde(skip)]
      &'hir BareFnTy<'hir>
  ),

  Never,

  Tup(
      #[serde(with = "Slice__TyDef")]
      &'hir [Ty<'hir>]
  ),

  Path(
      #[serde(with = "QPathDef")]
      QPath<'hir>
  ),

  OpaqueDef(
      #[serde(skip)]
      ItemId,

      #[serde(skip)]
      &'hir [GenericArg<'hir>],

      #[serde(skip)]
      bool
  ),

  TraitObject(
    #[serde(with = "Slice__PolyTraitRefDef")]
    &'hir [PolyTraitRef<'hir>],

    #[serde(with = "LifetimeDef")]
    &'hir Lifetime,

    #[serde(skip)]
    TraitObjectSyntax,
  ),

  // NOTE: after reading the documentation I'm still not sure what this is.
  Typeof(
      #[serde(skip)]
      AnonConst
  ),

  Infer,

  Err(#[serde(skip)] ErrorGuaranteed),
}

#[derive(Serialize)]
#[serde(remote = "QPath")]
pub enum QPathDef<'hir> {
    Resolved(
        #[serde(with = "Option__TyDef")]
        Option<&'hir Ty<'hir>>,

        #[serde(with = "PathDef")]
        &'hir Path<'hir>
    ),
    TypeRelative(
        #[serde(with = "TyDef")]
        &'hir Ty<'hir>,

        #[serde(with = "PathSegmentDef")]
        &'hir PathSegment<'hir>
    ),
    LangItem(
        #[serde(skip)]
        LangItem,

        #[serde(skip)]
        Span,
    ),
}

#[derive(Serialize)]
#[serde(remote = "TraitRef")]
pub struct TraitRefDef<'hir> {
  #[serde(with = "PathDef")]
  pub path: &'hir Path<'hir>,

  #[serde(skip)]
  pub hir_ref_id: HirId,
}

pub struct Option__TraitRefDef;
impl Option__TraitRefDef {
    fn serialize<'hir, S>(value: &Option<TraitRef<'hir>>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            None => NOOP.serialize(s),
            Some(ga) => TraitRefDef::serialize(ga, s),
        }
    }
}

#[derive(Serialize)]
#[serde(remote = "Path")]
pub struct PathDef<'hir, R = Res> {
  #[serde(skip)]
  pub span: Span,

  #[serde(skip)]
  pub res: R,

  #[serde(with = "Slice__PathSegmentDef")]
  pub segments: &'hir [PathSegment<'hir>],
}

#[derive(Serialize)]
#[serde(remote = "TypeBinding")]
pub struct TypeBindingDef<'hir> {
    #[serde(skip)]
    pub hir_id: HirId,

    #[serde(with = "IdentDef")]
    pub ident: Ident,

    #[serde(with = "GenericArgsDef")]
    pub gen_args: &'hir GenericArgs<'hir>,

    #[serde(with = "TypeBindingKindDef")]
    pub kind: TypeBindingKind<'hir>,

    #[serde(skip)]
    pub span: Span,
}

pub struct Slice__TypeBindingDef;
impl Slice__TypeBindingDef {
    fn serialize<'hir, S>(value: &[TypeBinding<'hir>], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'a>(
            #[serde(with = "TypeBindingDef")]
            &'a TypeBinding<'a>,
        );
        serialize_custom_seq! { Wrapper, s, value}
    }
}

#[derive(Serialize)]
#[serde(remote = "TypeBindingKind")]
pub enum TypeBindingKindDef<'hir> {
  Constraint {
      #[serde(with = "Slice__GenericBoundDef")]
      bounds: &'hir [GenericBound<'hir>]
  },
  Equality {
      #[serde(with = "TermDef")]
      term: Term<'hir>
  },
}

#[derive(Serialize)]
#[serde(remote = "Term")]
pub enum TermDef<'hir> {
    Ty(#[serde(with = "TyDef")] &'hir Ty<'hir>),
    Const(#[serde(with = "AnonConstDef")] AnonConst),
}

#[derive(Serialize)]
#[serde(remote = "Impl")]
pub struct ImplDef<'hir> {
    #[serde(skip)]
    pub unsafety: Unsafety,

    #[serde(with = "ImplPolarityDef")]
    pub polarity: ImplPolarity,

    #[serde(skip)]
    pub defaultness: Defaultness,

    #[serde(skip)]
    pub defaultness_span: Option<Span>,

    #[serde(with = "GenericsDef")]
    pub generics: &'hir Generics<'hir>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "Option__TraitRefDef")]
    pub of_trait: Option<TraitRef<'hir>>,

    #[serde(with = "TyDef")]
    pub self_ty: &'hir Ty<'hir>,

    #[serde(skip)]
    pub items: &'hir [ImplItemRef],
}

#[derive(Serialize)]
#[serde(remote = "ImplPolarity")]
pub enum ImplPolarityDef {
    Positive,
    Negative(#[serde(skip)] Span),
}

#[derive(Serialize)]
#[serde(remote = "Generics")]
pub struct GenericsDef<'hir> {
    #[serde(with = "Slice__GenericParamDef")]
    pub params: &'hir [GenericParam<'hir>],

    #[serde(with = "Slice__WherePredicateDef")]
    pub predicates: &'hir [WherePredicate<'hir>],

    #[serde(skip)]
    pub has_where_clause_predicates: bool,

    #[serde(skip)]
    pub where_clause_span: Span,

    #[serde(skip)]
    pub span: Span,
}

pub struct Slice__GenericParamDef;
impl Slice__GenericParamDef {
    fn serialize<'hir, S>(value: &[GenericParam<'hir>], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'a>(
            #[serde(with = "GenericParamDef")]
            &'a GenericParam<'a>,
        );
        serialize_custom_seq! { Wrapper, s, value}
    }
}

#[derive(Serialize)]
#[serde(remote = "GenericParam")]
pub struct GenericParamDef<'hir> {
    #[serde(skip)]
    pub hir_id: HirId,

    #[serde(skip)]
    pub def_id: LocalDefId,

    #[serde(with = "ParamNameDef")]
    pub name: ParamName,

    #[serde(skip)]
    pub span: Span,

    #[serde(skip)]
    pub pure_wrt_drop: bool,

    #[serde(with = "GenericParamKindDef")]
    pub kind: GenericParamKind<'hir>,

    #[serde(skip)]
    pub colon_span: Option<Span>,

    #[serde(skip)]
    pub source: GenericParamSource,
}

#[derive(Serialize)]
#[serde(remote = "ParamName")]
pub enum ParamNameDef {
    Plain(#[serde(with = "IdentDef")] Ident),
    Fresh,
    Error,
}

#[derive(Serialize)]
#[serde(remote = "GenericParamKind")]
pub enum GenericParamKindDef<'hir> {
    Lifetime {
        #[serde(skip)]
        kind: LifetimeParamKind,
    },
    Type {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(with = "Option__TyDef")]
        default: Option<&'hir Ty<'hir>>,

        #[serde(skip)]
        synthetic: bool,
    },
    Const {
        #[serde(with = "TyDef")]
        ty: &'hir Ty<'hir>,

        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(with = "Option__AnonConstDef")]
        default: Option<AnonConst>,

        #[serde(skip)]
        is_host_effect: bool,
    },
}

pub struct Slice__WherePredicateDef;
impl Slice__WherePredicateDef {
    fn serialize<'hir, S>(value: &[WherePredicate<'hir>], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'a>(
            #[serde(with = "WherePredicateDef")]
            &'a WherePredicate<'a>,
        );
        serialize_custom_seq! { Wrapper, s, value}
    }
}

#[derive(Serialize)]
#[serde(remote = "WherePredicate")]
pub enum WherePredicateDef<'hir> {
    BoundPredicate(
        #[serde(with = "WhereBoundPredicateDef")]
        WhereBoundPredicate<'hir>
    ),
    RegionPredicate(
        #[serde(with = "WhereRegionPredicateDef")]
        WhereRegionPredicate<'hir>
    ),
    EqPredicate(
        #[serde(with = "WhereEqPredicateDef")]
        WhereEqPredicate<'hir>
    ),
}

#[derive(Serialize)]
#[serde(remote = "WhereBoundPredicate")]
pub struct WhereBoundPredicateDef<'hir> {
    #[serde(skip)]
    pub hir_id: HirId,

    #[serde(skip)]
    pub span: Span,

    #[serde(skip)]
    pub origin: PredicateOrigin,

    #[serde(with = "Slice__GenericParamDef")]
    pub bound_generic_params: &'hir [GenericParam<'hir>],

    #[serde(with = "TyDef")]
    pub bounded_ty: &'hir Ty<'hir>,

    #[serde(with = "GenericBoundsDef")]
    pub bounds: GenericBounds<'hir>,
}

#[derive(Serialize)]
#[serde(remote = "WhereRegionPredicate")]
pub struct WhereRegionPredicateDef<'hir> {
    #[serde(skip)]
    pub span: Span,

    #[serde(skip)]
    pub in_where_clause: bool,

    #[serde(with = "LifetimeDef")]
    pub lifetime: &'hir Lifetime,

    #[serde(with = "GenericBoundsDef")]
    pub bounds: GenericBounds<'hir>,
}

#[derive(Serialize)]
#[serde(remote = "WhereEqPredicate")]
pub struct WhereEqPredicateDef<'hir> {
    #[serde(skip)]
    pub span: Span,

    #[serde(with = "TyDef")]
    pub lhs_ty: &'hir Ty<'hir>,

    #[serde(with = "TyDef")]
    pub rhs_ty: &'hir Ty<'hir>,
}

#[derive(Serialize)]
#[serde(remote = "GenericBound")]
pub enum GenericBoundDef<'hir> {
    Trait(
        #[serde(with = "PolyTraitRefDef")]
        PolyTraitRef<'hir>,
        #[serde(with = "TraitBoundModifierDef")]
        TraitBoundModifier
    ),
    Outlives(
        #[serde(with = "LifetimeDef")]
        &'hir Lifetime
    ),
}


pub type GenericBoundsDef = Slice__GenericBoundDef;
pub struct Slice__GenericBoundDef;
impl Slice__GenericBoundDef {
    fn serialize<'hir, S>(value: &[GenericBound<'hir>], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'a>(
            #[serde(with = "GenericBoundDef")]
            &'a GenericBound<'a>,
        );
        serialize_custom_seq! { Wrapper, s, value}
    }
}

#[derive(Serialize)]
#[serde(remote = "TraitBoundModifier")]
pub enum TraitBoundModifierDef {
    None,
    Negative,
    Maybe,
    MaybeConst,
    Const,
}


pub struct Slice__PolyTraitRefDef;
impl Slice__PolyTraitRefDef {
    fn serialize<'hir, S>(value: &[PolyTraitRef<'hir>], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wrapper<'a>(
            #[serde(with = "PolyTraitRefDef")]
            &'a PolyTraitRef<'a>,
        );
        serialize_custom_seq! { Wrapper, s, value}
    }
}

#[derive(Serialize)]
#[serde(remote = "PolyTraitRef")]
pub struct PolyTraitRefDef<'hir> {
    #[serde(with = "Slice__GenericParamDef")]
    pub bound_generic_params: &'hir [GenericParam<'hir>],

    #[serde(with = "TraitRefDef")]
    pub trait_ref: TraitRef<'hir>,

    #[serde(skip)]
    pub span: Span,
}

#[derive(Serialize)]
#[serde(remote = "AnonConst")]
pub struct AnonConstDef {
    #[serde(skip)]
    pub hir_id: HirId,

    #[serde(skip)]
    pub def_id: LocalDefId,

    #[serde(skip)]
    pub body: BodyId,
}

pub struct Option__AnonConstDef;
impl Option__AnonConstDef {
    fn serialize<'hir, S>(value: &Option<AnonConst>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            None => NOOP.serialize(s),
            Some(ga) => AnonConstDef::serialize(ga, s),
        }
    }
}

#[derive(Serialize)]
#[serde(remote = "Res")]
pub enum ResDef<Id = HirId> {
    Def(
        #[serde(skip)]
        DefKind,

        #[serde(skip)]
        DefId
    ),
    PrimTy(
        #[serde(skip)]
        PrimTy
    ),
    SelfTyParam {
        #[serde(skip)]
        trait_: DefId,
    },
    SelfTyAlias {
        #[serde(skip)]
        alias_to: DefId,
        #[serde(skip)]
        forbid_generic: bool,
        #[serde(skip)]
        is_trait_impl: bool,
    },
    SelfCtor(
        #[serde(skip)]
        DefId
    ),
    Local(
        #[serde(skip)]
        Id
    ),
    ToolMod,
    NonMacroAttr(
        #[serde(skip)]
        NonMacroAttrKind
    ),
    Err,
}
