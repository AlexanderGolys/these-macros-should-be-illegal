//! A tagged parameter envelope independent of each macro's body grammar.

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::{
    Ident, Token, parenthesized,
    parse::{Parse, ParseStream, Parser},
};

use super::NoArguments;

/// Reserved name of the leading parameter marker.
const MARKER: &str = "macro_args";

/// Recognizes the complete tagged prefix whose parameters must stay opaque.
pub(crate) fn opaque_prefix_length(tokens: &[TokenTree]) -> Option<usize> {
    match tokens {
        [
            TokenTree::Punct(at),
            TokenTree::Ident(marker),
            TokenTree::Group(arguments),
            TokenTree::Punct(end),
            ..,
        ] if at.as_char() == '@'
            && marker == MARKER
            && arguments.delimiter() == Delimiter::Parenthesis
            && end.as_char() == ';' =>
        {
            Some(4)
        }
        _ => None,
    }
}

/// Serializes parameters before an otherwise opaque macro input.
pub(crate) fn prepend(arguments: TokenStream, body: TokenStream) -> TokenStream {
    let marker = format_ident!("{MARKER}");
    quote!(@#marker(#arguments); #body)
}

/// Parses an optional leading envelope using the receiving macro's argument grammar.
pub(crate) fn parse_prefix<A: Parse>(input: ParseStream) -> syn::Result<Option<A>> {
    let lookahead = input.fork();
    if lookahead.parse::<Token![@]>().is_err()
        || !lookahead.parse::<Ident>().is_ok_and(|name| name == MARKER)
    {
        return Ok(None);
    }

    input.parse::<Token![@]>()?;
    input.parse::<Ident>()?;
    let arguments;
    parenthesized!(arguments in input);
    let parsed = arguments.parse()?;
    if !arguments.is_empty() {
        return Err(arguments.error("unexpected macro arguments"));
    }
    input.parse::<Token![;]>()?;
    if input.peek(Token![@]) {
        let lookahead = input.fork();
        lookahead.parse::<Token![@]>()?;
        if lookahead.parse::<Ident>().is_ok_and(|name| name == MARKER) {
            return Err(input.error("duplicate macro argument envelope"));
        }
    }
    Ok(Some(parsed))
}

/// Separates optional typed parameters from arbitrary, unparsed body tokens.
pub(crate) fn split<A: Parse>(input: TokenStream) -> syn::Result<(Option<A>, TokenStream)> {
    (|input: ParseStream| Ok((parse_prefix(input)?, input.parse()?))).parse2(input)
}

/// Runs a macro without parameters after validating and removing its envelope.
pub(crate) fn without_arguments(
    input: TokenStream,
    expand: impl FnOnce(TokenStream) -> TokenStream,
) -> TokenStream {
    split::<NoArguments>(input)
        .map(|(_, body)| expand(body))
        .unwrap_or_else(syn::Error::into_compile_error)
}

#[cfg(test)]
mod tests {
    //! Envelope boundary and validation tests.

    use super::*;

    /// The envelope never imposes a Rust grammar on body tokens.
    #[test]
    fn preserves_opaque_bodies() {
        let body = quote!(arbitrary @@@ { nested!(tokens) });
        let (arguments, rest) = split::<Ident>(prepend(quote!(chosen), body.clone())).unwrap();
        assert_eq!(arguments.unwrap(), "chosen");
        assert_eq!(rest.to_string(), body.to_string());
        let (arguments, rest) = split::<Ident>(body.clone()).unwrap();
        assert!(arguments.is_none());
        assert_eq!(rest.to_string(), body.to_string());
    }

    /// Malformed, repeated and unsupported parameters are diagnosed at the boundary.
    #[test]
    fn rejects_invalid_envelopes() {
        for source in [
            "@macro_args(); @macro_args(); body",
            "@macro_args() body",
            "@macro_args[anything]; body",
            "@macro_args(unexpected); body",
        ] {
            assert!(
                split::<NoArguments>(source.parse().unwrap()).is_err(),
                "{source}"
            );
        }
    }
}
