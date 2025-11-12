extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

// TODO: auto Resource
#[proc_macro_derive(PathMarker)]
pub fn derive_path_marker(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl PathMarker for #name {}
    };

    TokenStream::from(expanded)
}
