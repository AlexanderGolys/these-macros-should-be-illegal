//! Consumer tests for unique per-variant string discriminants.

use these_macros_should_be_illegal::discriminated_str;

/// Tokens whose payloads are deliberately irrelevant to their discriminants.
#[discriminated_str(name)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token<'a, T>
where
    T: Clone,
{
    /// A borrowed identifier.
    Ident(&'a str) = "ident",
    /// An arbitrary owned payload.
    Owned(T) = "owned",
    /// End of input.
    End = "end",
    /// A named payload.
    Located {
        /// One-based source line.
        line: usize,
        /// One-based source column.
        column: usize,
    } = "located",
    /// A conditionally absent variant.
    #[cfg(any())]
    Disabled = "disabled",
}

/// Forward string lookup remains available in constants.
const END_NAME: &str = Token::<'static, ()>::End.name();

/// The generated delegating constructor is usable in constants.
const END: Token<'static, ()> = Token!("end");

/// Verifies the forward string map for payload-bearing variants.
#[test]
fn maps_values_to_unique_discriminants() {
    let ident = Token::<()>::Ident("value");
    assert_eq!(ident.name(), "ident");

    let owned = Token::Owned(String::from("payload"));
    assert_eq!(owned.name(), "owned");

    assert_eq!(END_NAME, "end");
    assert_eq!(END.name(), "end");
}

/// Verifies literal-selected delegation to each constructor shape.
#[test]
fn constructs_values_from_literal_discriminants() {
    let ident: Token<'_, ()> = Token!("ident", "value");
    assert_eq!(ident, Token::Ident("value"));

    let owned: Token<'_, String> = Token!("owned", String::from("payload"));
    assert_eq!(owned, Token::Owned(String::from("payload")));

    let located: Token<'_, ()> = Token!("located", line: 3, column: 7);
    assert_eq!(located, Token::Located { line: 3, column: 7 });
}

/// The enum, inherent method, and constructor macro may all be block-local.
#[test]
fn expands_on_block_local_items() {
    #[discriminated_str(name)]
    enum LocalToken {
        /// The only local token.
        End = "end",
    }

    let token = LocalToken!("end");
    assert_eq!(token.name(), "end");
}
