//! Experimental procedural macros that are too entertaining not to try.

/// Procedural-macro implementations and their shared token preprocessing.
mod flust;

use flust::{
    discriminated_str_impl, excluded_macros_impl, expand_impl, literally_literal_string_impl,
    strutuct_impl,
};
use proc_macro::TokenStream;

/// Rewrites literal strings in exactly the supplied token stream.
#[rustfmt::skip] #[proc_macro]
pub fn literally_literal_string(input: TokenStream) -> TokenStream { literally_literal_string_impl(input.into()).into() }

#[doc = include_str!("../docs/discriminated_str.md")]
#[rustfmt::skip] #[proc_macro_attribute]
pub fn discriminated_str(arguments: TokenStream, item: TokenStream) -> TokenStream { discriminated_str_impl(arguments.into(), item.into()).into() }

/// Prevents transformations from descending into the listed macro invocations.
#[rustfmt::skip] #[proc_macro_attribute]
pub fn excluded_macros(arguments: TokenStream, item: TokenStream) -> TokenStream { excluded_macros_impl(arguments.into(), item.into()).into() }

/// Loads an out-of-line module and injects function-like macros around its body.
#[rustfmt::skip] #[proc_macro]
pub fn expand(input: TokenStream) -> TokenStream { expand_impl(input.into()).into() }

/// Generates one struct or enum together with all syntactically nested declarations.
#[rustfmt::skip] #[proc_macro]
pub fn strutuct(input: TokenStream) -> TokenStream { strutuct_impl(input.into()).into() }
