//! Construction and reflection of function-like macro invocation objects.

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::{Attribute, Error, Path, Token, parse::ParseStream, parse2};

/// A macro path together with attributes belonging to its invocation.
struct Invocation {
    /// Outer attributes emitted immediately before the invocation.
    attrs: Vec<Attribute>,
    /// Path identifying the invoked macro.
    path: Path,
}

/// Two macro paths and the opaque body around which they are reflected.
struct Reflection {
    /// Invocation that would conventionally appear on the outside.
    first: Invocation,
    /// Invocation reflected to the outside by this transformation.
    second: Invocation,
    /// Tokens retained opaquely inside both invocations.
    body: TokenStream,
}

impl Parse for Invocation {
    /// Parses outer attributes followed by one macro path.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            path: input.parse()?,
        })
    }
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

/// Constructs one attributed braced function-like macro invocation.
fn invoke_attributed(invocation: &Invocation, body: TokenStream) -> TokenStream {
    let attrs = &invocation.attrs;
    let path = &invocation.path;
    let invocation = invoke(path, body);
    quote!(#(#attrs)* #invocation)
}

/// Constructs one braced function-like macro invocation.
pub fn invoke(macro_path: &Path, body: TokenStream) -> TokenStream {
    quote!(#macro_path! { #body })
}

macro_docs! {
    /// Reflects two macro invocation objects around an opaque token-stream body.
    ///
    /// `reflect!(first, second; body)` constructs
    /// `second! { first! { body } }`. Consequently `second` expands before
    /// `first`, reversing the expansion order of `first! { second! { body } }`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use these_macros_should_be_illegal::reflect;
    /// macro_rules! add_one { ($value:expr) => { 1 + $value }; }
    /// macro_rules! double { ($value:expr) => { 2 * $value }; }
    /// assert_eq!(reflect!(add_one, double; 3), 8);
    /// ```
}

/// Reflects `first!(second!(body))` into `second!(first!(body))`.
pub fn reflect(input: TokenStream) -> TokenStream {
    parse2::<Reflection>(input)
        .map(|reflection| {
            let inner = invoke_attributed(&reflection.first, reflection.body);
            invoke_attributed(&reflection.second, inner)
        })
        .unwrap_or_else(Error::into_compile_error)
}

/// Invocation construction, reflection, paths, and opaque-body tests.
#[cfg(test)]
mod tests {
    use quote::quote;

    use super::{invoke, invoke_attributed, reflect};

    /// Invocation is a macro path paired with an opaque token stream.
    #[test]
    fn constructs_an_invocation_object() {
        let invocation = syn::parse_quote!(
            #[cfg(test)]
            some::transform
        );
        assert_eq!(
            invoke_attributed(&invocation, quote!(a + nested!(b))).to_string(),
            quote!(
                #[cfg(test)]
                some::transform! { a + nested!(b) }
            )
            .to_string()
        );
    }

    /// The shared path-only constructor remains available to other transforms.
    #[test]
    fn constructs_an_unattributed_invocation() {
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
            reflect(quote!(
                #[cfg(feature = "first")] first,
                #[cfg(feature = "second")] some::second;
                a + nested!(b)
            ))
            .to_string(),
            quote!(
                #[cfg(feature = "second")]
                some::second! {
                    #[cfg(feature = "first")]
                    first! { a + nested!(b) }
                }
            )
            .to_string()
        );
    }
}
