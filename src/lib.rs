//! Experimental procedural macros that are too entertaining not to try.

/// Procedural-macro implementations and their shared token preprocessing.
mod flust;
/// Implements qualification of common unqualified standard-library types.
mod mbe;

use flust::{
    discriminated_str_impl, excluded_macros_impl, expand_impl, literally_literal_string_impl,
    strutuct_impl,
};
use mbe::qualify_common as qualify_common_impl;
use proc_macro::TokenStream;

/// Rewrites literal strings in exactly the supplied token stream.
#[proc_macro]
pub fn literally_literal_string(input: TokenStream) -> TokenStream {
    literally_literal_string_impl(input.into()).into()
}

/// Qualifies common unqualified standard-library types inside one Rust type.
///
/// Nested common types are qualified recursively, while an already qualified
/// path is preserved deliberately.
///
/// ```
/// use these_macros_should_be_illegal::qf;
///
/// let value: qf!(Option<Vec<String>>) = Some(vec![String::from("less punctuation")]);
/// assert!(value.is_some());
/// ```
#[proc_macro]
pub fn qf(input: TokenStream) -> TokenStream {
    qualify_common_impl(input.into()).into()
}

#[doc = include_str!("../docs/discriminated_str.md")]
#[proc_macro_attribute]
pub fn discriminated_str(arguments: TokenStream, item: TokenStream) -> TokenStream {
    discriminated_str_impl(arguments.into(), item.into()).into()
}

/// Prevents transformations from descending into the listed macro invocations.
#[proc_macro_attribute]
pub fn excluded_macros(arguments: TokenStream, item: TokenStream) -> TokenStream {
    excluded_macros_impl(arguments.into(), item.into()).into()
}

/// Loads an out-of-line module and injects function-like macros around its body.
#[proc_macro]
pub fn expand(input: TokenStream) -> TokenStream {
    expand_impl(input.into()).into()
}

/// Generates one struct or enum together with all syntactically nested declarations.
#[proc_macro]
pub fn strutuct(input: TokenStream) -> TokenStream {
    strutuct_impl(input.into()).into()
}
