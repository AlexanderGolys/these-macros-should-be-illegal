//! Attribute bridge into the common function-like macro parameter envelope.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, ItemMacro, parse2};

use crate::helpers::arguments::input::prepend;

/// Prepends opaque arguments to one item-position function-like invocation.
pub(crate) fn macro_args(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<ItemMacro>(item).and_then(|mut invocation| {
        if invocation.ident.is_some() {
            return Err(Error::new_spanned(
                invocation,
                "`macro_args` expects a function-like macro invocation",
            ));
        }
        invocation.mac.tokens = prepend(arguments, invocation.mac.tokens);
        Ok(quote!(#invocation))
    });
    result.unwrap_or_else(Error::into_compile_error)
}
