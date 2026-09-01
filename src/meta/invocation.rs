//! Construction and reflection of function-like macro invocation objects.

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::{Error, Path, Token, parse::ParseStream, parse2};

/// Two macro paths and the opaque body around which they are reflected.
struct Reflection {
    /// Invocation that would conventionally appear on the outside.
    first: Path,
    /// Invocation reflected to the outside by this transformation.
    second: Path,
    /// Tokens retained opaquely inside both invocations.
    body: TokenStream,
}

impl Parse for Reflection {
    /// Parses `first, second; body` without imposing a grammar on the body.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let first = input.parse()?;
        input.parse::<Token![,]>()?;
        let second = input.parse()?;
        input.parse::<Token![;]>()?;
        let body = input.parse()?;
        Ok(Self {
            first,
            second,
            body,
        })
    }
}

/// Constructs one braced function-like macro invocation.
pub(crate) fn invoke(macro_path: &Path, body: TokenStream) -> TokenStream {
    quote!(#macro_path! { #body })
}

/// Reflects `first!(second!(body))` into `second!(first!(body))`.
pub(crate) fn reflect(input: TokenStream) -> TokenStream {
    parse2::<Reflection>(input)
        .map(|reflection| {
            let inner = invoke(&reflection.first, reflection.body);
            invoke(&reflection.second, inner)
        })
        .unwrap_or_else(Error::into_compile_error)
}

/// Invocation construction, reflection, paths, and opaque-body tests.
#[cfg(test)]
mod tests {
    use quote::quote;

    use super::{invoke, reflect};

    /// Invocation is a macro path paired with an opaque token stream.
    #[test]
    fn constructs_an_invocation_object() {
        let path = syn::parse_quote!(some::transform);
        assert_eq!(
            invoke(&path, quote!(a + nested!(b))).to_string(),
            quote!(some::transform! { a + nested!(b) }).to_string()
        );
    }

    /// Reflection exchanges the two invocation nodes without touching the body.
    #[test]
    fn reflects_two_macro_invocations() {
        assert_eq!(
            reflect(quote!(first, some::second; a + nested!(b))).to_string(),
            quote!(some::second! { first! { a + nested!(b) } }).to_string()
        );
    }
}
