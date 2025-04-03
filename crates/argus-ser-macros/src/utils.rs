use proc_macro2::Span;
use syn::{
  parse::{Parse, ParseStream},
  Attribute, Expr, Generics, Ident, Item, ItemEnum, ItemStruct, Lifetime,
  LitStr, Path, Token, WherePredicate,
};
use uuid::Uuid;

pub fn remove_def_annotation(s: &str) -> &str {
  let len = s.len();
  if len >= 3 || "Def" != &s[len - 3 ..] {
    &s[.. len - 3]
  } else {
    panic!("The final three characters of the type must be `Def`");
  }
}

pub struct Remote(Path, String);

impl Remote {
  pub fn name(value: Ident) -> Self {
    Self(Path::from(value.clone()), value.to_string())
  }

  pub fn path(&self) -> &Path {
    &self.0
  }

  pub fn lit(&self) -> &String {
    &self.1
  }
}

impl syn::parse::Parse for Remote {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    while !input.is_empty() {
      let key: Ident = input.call(syn::ext::IdentExt::parse_any)?;
      if key.to_string().eq("remote") {
        let _eq: Token![=] = input.parse()?;
        let ty_str: LitStr = input.parse()?;
        let ty: Path = ty_str.parse()?;
        return Ok(Remote(ty, ty_str.value()));
      }
    }

    Err(syn::Error::new(input.span(), "Missing `remote` attribute"))
  }
}

impl TryFrom<&Attribute> for Remote {
  type Error = syn::Error;

  fn try_from(attr: &syn::Attribute) -> syn::Result<Self> {
    attr.parse_args()
  }
}

pub fn remote_info(attrs: &[syn::Attribute]) -> Option<Remote> {
  attrs
    .iter()
    .filter(|a| a.path().is_ident("serde") || a.path().is_ident("argus"))
    .find_map(|attr| Remote::try_from(attr).ok())
}

pub struct ItemLikeFields<'a> {
  pub name: &'a Ident,
  pub attrs: &'a [Attribute],
  pub generics: &'a Generics,
}

pub fn item_like_fields<'a>(input: &'a Item) -> ItemLikeFields<'a> {
  match &input {
    Item::Struct(ItemStruct {
      ident,
      attrs,
      generics,
      ..
    })
    | Item::Enum(ItemEnum {
      ident,
      attrs,
      generics,
      ..
    }) => ItemLikeFields {
      name: ident,
      attrs,
      generics,
    },
    _ => panic!("Expected a `struct` or `enum` definition"),
  }
}

pub struct ArgusIdentifiers {
  pub name: Ident,
  pub poly: Ident,
  pub maybe: Ident,
  pub slice: Ident,
  pub remote: Remote,
  pub generics: Generics,
}

impl From<ItemLikeFields<'_>> for ArgusIdentifiers {
  fn from(
    ItemLikeFields {
      name,
      attrs,
      generics,
      ..
    }: ItemLikeFields<'_>,
  ) -> Self {
    let span = name.span();
    let poly = Ident::new(&format!("Poly{name}"), span);
    let maybe = Ident::new(&format!("Maybe{name}"), span);
    let slice = Ident::new(&format!("{}s", name), span);
    let remote = remote_info(attrs).unwrap_or_else(|| {
      let name = &name.to_string();
      let name = Ident::new(remove_def_annotation(name), span);
      Remote::name(name)
    });

    ArgusIdentifiers {
      name: name.clone(),
      poly,
      slice,
      maybe,
      remote,
      generics: generics.clone(),
    }
  }
}

pub fn argus_identifiers(input: &Item) -> ArgusIdentifiers {
  item_like_fields(input).into()
}

pub fn gensym_lifetime() -> Lifetime {
  let unique_name = format!("__t{}", Uuid::new_v4().as_simple());
  let ident = Ident::new_raw(&unique_name, Span::call_site());
  Lifetime {
    apostrophe: Span::call_site(),
    ident,
  }
}

pub fn introduce_outlives(generics: &mut Generics) -> Lifetime {
  let lifetime = gensym_lifetime();

  generics
    .params
    .push(syn::GenericParam::Lifetime(syn::LifetimeParam::new(
      lifetime.clone(),
    )));

  // If there is no where clause, create one.
  if generics.where_clause.is_none() {
    generics.where_clause = Some(syn::parse_quote!(where));
  }

  // Add lifetime constraints to the where clause.
  let new_outlives_bounds = generics
    .lifetimes()
    .map(|param| {
      let lifetime_ident = &param.lifetime;
      let bound: WherePredicate = syn::parse_quote!(#lifetime_ident: #lifetime);
      bound
    })
    .collect::<Vec<WherePredicate>>();

  if let Some(where_clause) = &mut generics.where_clause {
    where_clause.predicates.extend(new_outlives_bounds);
  }

  lifetime
}

pub struct SerializeCustomSeqArgs {
  pub wrap: Ident,
  pub serializer: Expr,
  pub value: Expr,
}

impl Parse for SerializeCustomSeqArgs {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let wrap = input.parse()?;
    input.parse::<Token![,]>()?;

    let serializer = input.parse()?;
    input.parse::<Token![,]>()?;

    let value = input.parse()?;
    Ok(SerializeCustomSeqArgs {
      wrap,
      serializer,
      value,
    })
  }
}
