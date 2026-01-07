use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, DeriveInput, Lit, Meta};

#[proc_macro_derive(Path, attributes(file))]
pub fn derive_path(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Parse the #[file("...")] attribute
    let template =
        parse_path_attribute(&input.attrs).expect("Missing or invalid #[file(\"...\")] attribute");

    let expanded = quote! {
        impl bevy_paths::TypedPath for #name {
            fn template() -> &'static str {
                #template
            }
        }
    };

    TokenStream::from(expanded)
}

fn parse_path_attribute(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("file") {
            if let Meta::List(list) = &attr.meta {
                // Handle #[file("...")]
                if let Ok(Lit::Str(lit_str)) = syn::parse2::<Lit>(list.tokens.clone()) {
                    return Some(lit_str.value());
                }
            } else if let Meta::NameValue(nv) = &attr.meta {
                // Handle #[file = "..."] (alternative style)
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        return Some(lit_str.value());
                    }
                }
            }
        }
    }
    None
}
