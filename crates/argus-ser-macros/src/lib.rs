pub(crate) mod utils;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item};
use utils::*;

#[proc_macro_attribute]
pub fn argus(_attr: TokenStream, item: TokenStream) -> TokenStream {
  item
}

#[proc_macro]
pub fn serialize_custom_seq(input: TokenStream) -> TokenStream {
  let SerializeCustomSeqArgs {
    wrap,
    serializer,
    value,
  } = parse_macro_input!(input as SerializeCustomSeqArgs);

  TokenStream::from(quote! {{
     use serde::ser::SerializeSeq;
     let mut seq = #serializer.serialize_seq(Some(#value.len()))?;
     for e in #value.into_iter() {
         seq.serialize_element(&(#wrap)(e))?;
     }
     seq.end()
  }})
}

#[proc_macro_derive(Many)]
pub fn create_slice_wrapper(item: TokenStream) -> TokenStream {
  let ast = parse_macro_input!(item as Item);
  let argus: ArgusIdentifiers = argus_identifiers(&ast);

  let mut generics_outlive_a = argus.generics.clone();
  let life_a = introduce_outlives(&mut generics_outlive_a);

  let ArgusIdentifiers {
    name,
    remote,
    slice: slice_name,
    generics,
    ..
  } = argus;
  let name_str = name.to_string();
  let raw = remote.path();

  TokenStream::from(quote! {
      pub struct #slice_name;
      impl #slice_name {
        pub fn serialize<S: serde::Serializer>(
          value: &[#raw],
          s: S
        ) -> Result<S::Ok, S::Error> {
          #[derive(serde::Serialize)]
          struct Wrapper #generics_outlive_a (
            #[serde(with = #name_str)]
            & #life_a #raw #generics
          );
          crate::serialize_custom_seq!(Wrapper, s, value)
        }
      }
  })
}

#[proc_macro_derive(Poly)]
pub fn create_poly_wrapper(item: TokenStream) -> TokenStream {
  let ast = parse_macro_input!(item as Item);

  let ArgusIdentifiers {
    name, remote, poly, ..
  } = argus_identifiers(&ast);

  let raw = remote.path();
  let remote_path_str = remote.lit();
  let ts_ty_str = remote.path().segments.last().unwrap().ident.to_string();
  let name_str = name.to_string();
  let ts_str = format!("Poly{}", remove_def_annotation(&name_str));

  TokenStream::from(quote! {
      #[derive(Clone, Debug, serde::Serialize)]
      #[serde(rename_all = "camelCase")]
      #[argus(remote = #remote_path_str)]
      #[cfg_attr(feature = "testing", derive(ts_rs::TS))]
      #[cfg_attr(feature = "testing", ts(export, rename = #ts_str))]
      pub struct #poly<'tcx> {
          #[serde(with = #name_str)]
          #[cfg_attr(feature = "testing", ts(type = #ts_ty_str))]
          value: #raw<'tcx>,

          #[serde(with = "crate::ty::BoundVariableKindDefs")]
          #[cfg_attr(feature = "testing", ts(type = "BoundVariableKind[]"))]
          bound_vars: &'tcx rustc_middle::ty::List<rustc_middle::ty::BoundVariableKind>,
      }

      impl<'tcx> #poly<'tcx> {
          pub fn new(value: &rustc_middle::ty::Binder<'tcx, #raw<'tcx>>) -> Self {
              let value = value.clone();
              Self {
                  bound_vars: value.bound_vars(),
                  value: value.skip_binder(),
              }
          }

          pub fn serialize<S: serde::Serializer>(
              value: &rustc_middle::ty::Binder<'tcx, #raw<'tcx>>,
              s: S,
          ) -> Result<S::Ok, S::Error> {
              Self::new(value).serialize(s)
          }
      }
  })
}

#[proc_macro_derive(Maybe)]
pub fn create_maybe_wrapper(item: TokenStream) -> TokenStream {
  let ast = parse_macro_input!(item as Item);

  let ArgusIdentifiers {
    name,
    remote,
    maybe,
    ..
  } = argus_identifiers(&ast);

  let raw = remote.path();

  TokenStream::from(quote! {
      pub struct #maybe;
    impl #maybe {
        pub fn serialize<S>(value: &Option<#raw>, s: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            match value {
            None => s.serialize_none(),
            Some(ty) => #name::serialize(ty, s),
            }
        }
        }
  })
}
