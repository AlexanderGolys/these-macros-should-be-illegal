//! Experimental procedural macros that are too entertaining not to try.

/// Whole-stream grammar extensions applied recursively before parsing.
mod flust;
/// Reusable plumbing for composing and configuring macros.
mod helpers;
/// Shape-aware local syntax extensions and replacement grammars.
mod metalust;
/// Small conveniences over otherwise ordinary Rust syntax.
mod rast;

use flust::{literally_literal_string_impl, shared_match_arms_impl};
use helpers::{excluded_macros_impl, expand_impl, forward_attributes_impl};
use metalust::{discriminated_str_impl, strutuct_impl};
use proc_macro::TokenStream;
use rast::qualify_common_impl;

/// Rewrites literal strings in exactly the supplied token stream.
#[proc_macro]
pub fn literally_literal_string(input: TokenStream) -> TokenStream {
    literally_literal_string_impl(input.into()).into()
}

/// Clones a shared match-arm RHS for alternatives separated by `||`.
///
/// Parenthesize `(pattern if guard)` when one alternative has its own guard.
///
/// ```
/// use these_macros_should_be_illegal::shared_match_arms;
///
/// enum Value {
///     Number(u32),
///     Character(char),
/// }
///
/// let value = Value::Number(3);
/// let text = shared_match_arms! {
///     match value {
///         Value::Number(value) || Value::Character(value) => value.to_string(),
///     }
/// };
///
/// assert_eq!(text, "3");
/// ```
#[proc_macro]
pub fn shared_match_arms(input: TokenStream) -> TokenStream {
    shared_match_arms_impl(input.into()).into()
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

#[doc = include_str!("../docs/forward-attributes.md")]
#[proc_macro_attribute]
pub fn forward_attributes(arguments: TokenStream, item: TokenStream) -> TokenStream {
    forward_attributes_impl(arguments.into(), item.into()).into()
}

#[doc = include_str!("../docs/strutuct.md")]
#[proc_macro]
pub fn strutuct(input: TokenStream) -> TokenStream {
    strutuct_impl(input.into()).into()
}

/// Expands exactly like [`strutuct!`](strutuct).
#[proc_macro]
pub fn emmun(input: TokenStream) -> TokenStream {
    strutuct_impl(input.into()).into()
}
