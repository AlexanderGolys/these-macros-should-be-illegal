//! Names for generated parameters that cannot collide with anything written beside them.

use proc_macro2::{TokenStream, TokenTree};

/// Collects every identifier written in a token stream, lifetimes' names included.
///
/// A generated parameter shares its scope with all of them: a declared parameter of any
/// kind, the declared type's own name, and every type named in a bound would each be
/// shadowed or redeclared by a parameter of the same name.
pub(crate) fn written_identifiers(tokens: TokenStream) -> Vec<String> {
    let mut identifiers = Vec::new();
    for tree in tokens {
        match tree {
            TokenTree::Ident(ident) => identifiers.push(ident.to_string()),
            TokenTree::Group(group) => identifiers.extend(written_identifiers(group.stream())),
            TokenTree::Punct(_) | TokenTree::Literal(_) => {}
        }
    }

    identifiers
}

/// Picks `base`, or the first numbered variant of it, that `taken` does not contain.
pub(crate) fn fresh_name(base: &str, taken: &[String]) -> String {
    std::iter::once(base.to_owned())
        .chain((2..).map(|index| format!("{base}{index}")))
        .find(|candidate| !taken.contains(candidate))
        .expect("an unbounded sequence of distinct candidates contains an unused one")
}
