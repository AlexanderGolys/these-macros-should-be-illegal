//! Helper attribute that adds macro names to the opaque preprocessing set.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, ItemMacro, parse2};

use crate::helpers::preprocessing::{ExcludedMacros, ExpansionConfig, split_config_prefix};

macro_docs! {
    /// Prevents transformations from descending into the listed macro invocations.
    ///
    /// # Examples
    ///
    /// ```
    /// use these_macros_should_be_illegal::{excluded_macros, literally_literal_string};
    /// macro_rules! raw_tokens { ($($tokens:tt)*) => { "borrowed" }; }
    ///
    /// #[excluded_macros(raw_tokens)]
    /// literally_literal_string! {
    ///     fn value() -> &'static str { raw_tokens!(@@"untouched") }
    /// }
    ///
    /// fn main() {
    ///     assert_eq!(value(), "borrowed");
    /// }
    /// ```
}

/// Adds excluded macro names to an invocation's shared preprocessing envelope.
pub fn excluded_macros(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<ExcludedMacros>(arguments)
        .and_then(|excluded| configure(ExpansionConfig::excluding(excluded), item));

    result.unwrap_or_else(Error::into_compile_error)
}

/// Merges exclusions into an item-position macro invocation's configuration envelope.
fn configure(mut config: ExpansionConfig, item: TokenStream) -> syn::Result<TokenStream> {
    let mut invocation = parse2::<ItemMacro>(item)?;
    let (existing, tokens) = split_config_prefix(invocation.mac.tokens)?;
    config.merge(existing);
    invocation.mac.tokens = config.configure_input(tokens);

    Ok(quote!(#invocation))
}
