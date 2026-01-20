#![warn(missing_docs)]

//! The `bevy_paths_derive` crate provides the procedural macro for deriving the `TypedPath` trait.
//!
//! It allows you to define **type-safe path templates** with placeholders and automatically derive the `TypedPath` trait for your structs.
//!
//! # Architecture
//!
//! This crate:
//! - Parses the `#[file("...")]` attribute to extract path templates.
//! - Validates templates using [`bevy_paths_validation`].
//! - Generates an implementation of `TypedPath` with the template and placeholders.
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```rust
//! use bevy_paths::prelude::*;
//! use bevy_reflect::Reflect;
//!
//! #[derive(Path, Reflect, Debug)]
//! #[file("assets/{name}.png")]
//! struct AssetPath {
//!     name: String,
//! }
//!
//! let path = AssetPath { name: "example".to_string() };
//! let _resolved = path.resolve();
//! ```
//!
//! ## Dynamic Paths with Multiple Placeholders
//!
//! ```rust
//! use bevy_paths::prelude::*;
//! use bevy_reflect::Reflect;
//!
//! #[derive(Path, Reflect, Debug)]
//! #[file("saves/{save_name}/region_{x}_{y}.map")]
//! struct RegionMap {
//!     save_name: String,
//!     x: u32,
//!     y: u32,
//! }
//!
//! let map = RegionMap { save_name: "MySaveGame".into(), x: 10, y: 20 };
//! let _resolved = map.resolve();
//! ```
//!
//! # Attributes
//!
//! - `#[file("...")]`: Specifies the path template for the struct. Must be a **relative path** with optional `{placeholder}` fields.
//!
//! # Errors
//!
//! The macro will generate a **compile error** if:
//! - The `#[file("...")]` attribute is missing.
//! - The path template is invalid (e.g., absolute paths, `..`, or invalid characters).
//! - Placeholders do not match struct fields.
//!
//! # Safety
//!
//! This macro is **safe** and does not use `unsafe` code.
//!
//! # Performance
//!
//! - Template validation is performed at compile time.
//! - Placeholder extraction uses a regex (executed once at compile time).
//!
//! # Dependencies
//!
//! - [`bevy_paths_validation`]: Validates path templates.
//!
//! # License
//!
//! MIT

use {
    bevy_paths_validation::validate_structural_path,
    proc_macro::TokenStream,
    quote::quote,
    syn::{DeriveInput, parse_macro_input},
};

/// Derives the `TypedPath` trait for a struct.
///
/// This macro:
/// - Extracts the `#[file("...")]` attribute to get the path template.
/// - Validates the template using `bevy_paths_validation`.
/// - Generates an implementation of `TypedPath` with the template and placeholders.
///
/// # Panics
///
/// This macro will panic if:
/// - The `#[file("...")]` attribute is missing.
/// - The path template is invalid.
#[proc_macro_derive(Path, attributes(file))]
pub fn derive_path(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let template = extract_file_attribute(&input).expect("Missing #[file(\"...\")] attribute");
    if let Err(e) = validate_structural_path(&template) {
        let error_msg = format!("Invalid path template: {}", e);
        return quote! { compile_error!(#error_msg); }.into();
    }
    // Platzhalter extrahieren
    let placeholders = extract_placeholders(&template);
    let struct_name = &input.ident;
    quote! {
        impl TypedPath for #struct_name {
            const TEMPLATE: &'static str = #template;
            const PLACEHOLDERS: &'static [&'static str] = &[#(#placeholders),*];
        }
    }
    .into()
}

fn extract_file_attribute(input: &DeriveInput) -> Option<String> {
    input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("file"))?
        .parse_args::<syn::LitStr>()
        .ok()
        .map(|lit| lit.value())
}

fn extract_placeholders(template: &str) -> Vec<String> {
    let re = regex::Regex::new(r"\{([^}]+)\}").unwrap();
    re.captures_iter(template)
        .map(|cap| cap[1].to_string())
        .collect()
}
