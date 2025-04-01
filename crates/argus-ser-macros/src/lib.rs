use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, token::Token, Ident, ItemStruct};

#[proc_macro_derive(Poly)]
pub fn create_poly_wrapper(item: TokenStream) -> TokenStream {
  // Parse the input tokens into a syntax tree
  let ast = parse_macro_input!(item as ItemStruct);

  // Build the impl
  let name = &ast.ident;
  let raw_name = Ident::new(&"TODO", name.span());
  let poly_name = Ident::new(&format!("Poly{name}"), name.span());
  let binder_name = Ident::new(&format!("Binder__{name}"), name.span());

  TokenStream::from(quote! {
      type #poly_name<'tcx> = #binder_name<'tcx>;
      struct #binder_name {
          value: rustc_middle::ty::#raw_name,
          bound_vars: &'tcx rustc_middle::ty::List<rustc_middle::ty::BoundVariableKind>,
      }
  })
}
