//! Helper for forwarding attributes into arbitrary function-like invocations.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, ItemMacro, parse2};

use crate::helpers::arguments::NoArguments;

macro_docs! {
    /// Moves the outer attributes of an item-position macro invocation into its input.
    ///
    /// The receiving function-like macro then decides what the attributes mean.
    ///
    /// # Examples
    ///
    /// ```
    /// use these_macros_should_be_illegal::{forward_attributes, strutuct};
    ///
    /// #[forward_attributes]
    /// #[derive(Debug, PartialEq)]
    /// strutuct! {
    ///     State { Ready, Waiting }
    /// }
    ///
    /// fn main() {
    ///     assert_eq!(format!("{:?}", State::Waiting), "Waiting");
    /// }
    /// ```
}

/// Moves an invocation's remaining outer attributes into its opaque input.
pub fn forward_attributes(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<NoArguments>(arguments).and_then(|NoArguments| forward(item));
    result.unwrap_or_else(Error::into_compile_error)
}

/// Rebuilds one function-like invocation with an attribute/input boundary.
fn forward(item: TokenStream) -> syn::Result<TokenStream> {
    let mut invocation = parse2::<ItemMacro>(item)?;
    if invocation.ident.is_some() {
        return Err(Error::new_spanned(
            invocation,
            "`forward_attributes` expects a function-like macro invocation",
        ));
    }

    let attrs = std::mem::take(&mut invocation.attrs);
    let body = std::mem::take(&mut invocation.mac.tokens);
    invocation.mac.tokens = quote!(#(#attrs)* ; #body);

    Ok(quote!(#invocation))
}

#[cfg(test)]
mod tests {
    //! Unit tests for the attribute-to-input envelope.

    use super::*;

    /// Moves attributes without parsing deliberately non-Rust macro input.
    #[test]
    fn forwards_attributes_into_opaque_input() {
        let input = r#"
            #[derive(Debug)]
            #[option(any tokens at all)]
            path::macro_name! { Root definitely is not Rust @@@ }
        "#
        .parse()
        .unwrap();
        let output = forward(input).unwrap().to_string();

        assert!(output.starts_with("path :: macro_name !"));
        assert!(output.contains("# [derive (Debug)]"));
        assert!(output.contains("# [option (any tokens at all)]"));
        assert!(
            output.contains("; Root definitely is not Rust @@@"),
            "{output}"
        );
    }

    /// Retains the delimiter and trailing semicolon of an item macro invocation.
    #[test]
    fn preserves_invocation_shell() {
        let input = "#[cfg(test)] make_items!(private syntax);".parse().unwrap();
        let output = forward(input).unwrap().to_string();

        assert_eq!(output, "make_items ! (# [cfg (test)] ; private syntax) ;");
    }
}
