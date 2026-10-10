//! Views through the invisible groups around fragments forwarded by declarative macros.
//!
//! A `macro_rules!` forwarding `$value:expr` or `$value:ty` wraps it in an invisible
//! group, which syn parses as `Expr::Group` or `Type::Group`. Matching on the written
//! shape must look through those groups.

use syn::{Expr, Type};

/// The expression as written, inside any invisible groups.
pub(crate) fn expr(expression: &Expr) -> &Expr {
    match expression {
        Expr::Group(group) => expr(&group.expr),
        expression => expression,
    }
}

/// The type as written, inside any invisible groups.
pub(crate) fn ty(written: &Type) -> &Type {
    match written {
        Type::Group(group) => ty(&group.elem),
        written => written,
    }
}

/// Invisible-group unwrapping tests.
#[cfg(test)]
mod tests {
    use proc_macro2::{Delimiter, Group, TokenStream};
    use quote::quote;
    use syn::{Expr, Type, parse2};

    /// Wraps tokens the way `macro_rules!` forwards a fragment.
    fn invisible(tokens: TokenStream) -> TokenStream {
        Group::new(Delimiter::None, tokens).into_token_stream()
    }

    use quote::ToTokens;

    /// Forwarded literals and types are recognized by their written shape.
    #[test]
    fn looks_through_forwarded_fragments() {
        let literal: Expr = parse2(invisible(invisible(quote!("a")))).unwrap();
        assert!(matches!(super::expr(&literal), Expr::Lit(_)));

        let tuple: Type = parse2(invisible(quote!((u8, u16)))).unwrap();
        assert!(matches!(super::ty(&tuple), Type::Tuple(_)));
    }
}
