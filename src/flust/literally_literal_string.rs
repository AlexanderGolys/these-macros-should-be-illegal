//! Implementation of the deliberately non-Rust `@@"literal"` string syntax.

use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use syn::{Error, LitStr, parse2};

#[cfg(test)]
use crate::helpers::preprocessing::ExpansionConfig;
use crate::helpers::preprocessing::{is_joint_punctuation, is_punctuation, split_config_prefix};

/// Rewrites extended literal-string syntax in a procedural macro's input.
pub(crate) fn literally_literal_string(input: TokenStream) -> TokenStream {
    expand(input).unwrap_or_else(Error::into_compile_error)
}

/// Removes shared configuration and rewrites all eligible literal-string tokens.
fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let (config, input) = split_config_prefix(input)?;
    Ok(config.rewrite(input, &literal_string_rewrite))
}

/// Replaces a leading joint `@@` and string literal with an owned string expression.
fn literal_string_rewrite(tokens: &[TokenTree]) -> Option<(usize, TokenStream)> {
    if is_joint_punctuation(tokens.first()?, '@')
        && matches!(tokens.get(1), Some(token) if is_punctuation(token, '@'))
    {
        let literal = tokens.get(2).and_then(string_literal)?;
        return Some((3, owned_string(literal)));
    }

    None
}

/// Parses a single token as a Rust string literal.
fn string_literal(token: &TokenTree) -> Option<LitStr> {
    parse2(token.clone().into()).ok()
}

/// Builds the standard-library expression used for an owned string literal.
fn owned_string(literal: LitStr) -> TokenStream {
    quote!(::std::string::String::from(#literal))
}

#[cfg(test)]
mod tests {
    //! Unit tests for literal-string rewriting and preprocessing integration.

    use super::*;

    /// Lexes deliberately invalid Rust syntax with proc-macro2's fallback lexer.
    fn flust(input: &str) -> TokenStream {
        input.parse().unwrap()
    }

    /// Runs literal-string rewriting with an empty shared configuration.
    fn rewrite_all(input: TokenStream) -> TokenStream {
        ExpansionConfig::default().rewrite(input, &literal_string_rewrite)
    }

    /// Rewrites a direct `@@"literal"` token sequence.
    #[test]
    fn turns_quoted_string_token_pair_into_string() {
        let input = flust(r#"@@"horrible""#);
        let expected = quote!(::std::string::String::from("horrible"));

        assert_eq!(rewrite_all(input).to_string(), expected.to_string());
    }

    /// Recurses through ordinary token groups.
    #[test]
    fn rewrites_inside_groups() {
        let input = flust(r#"{ @@"nested" }"#);

        assert!(rewrite_all(input).to_string().contains("String :: from"));
    }

    /// Leaves attribute contents completely opaque.
    #[test]
    fn leaves_attributes_opaque() {
        let input = flust(r#"#[@@"untouched"]"#);
        let original = input.to_string();

        assert_eq!(rewrite_all(input).to_string(), original);
    }

    /// Rewrites declarative macro bodies unless `macro_rules` is excluded.
    #[test]
    fn rewrites_declarative_macro_groups() {
        let input = flust(r#"macro_rules! make_string { @@"untouched" }"#);
        let expected = quote! {
            macro_rules! make_string {
                ::std::string::String::from("untouched")
            }
        };

        assert_eq!(rewrite_all(input).to_string(), expected.to_string());
    }

    /// Continues traversal after a parenthesized declarative macro definition.
    #[test]
    fn continues_rewriting_after_declarative_macro_definitions() {
        let input = flust(r#"macro_rules! make_string (@@"untouched"); @@"rewritten""#);
        let expected = quote! {
            macro_rules! make_string (
                ::std::string::String::from("untouched")
            );
            ::std::string::String::from("rewritten")
        };

        assert_eq!(rewrite_all(input).to_string(), expected.to_string());
    }

    /// Preserves an excluded declarative macro while rewriting following tokens.
    #[test]
    fn leaves_excluded_declarative_macro_opaque() {
        let input = flust(r#"macro_rules! make_string { @@"untouched" } @@"rewritten""#);
        let config: ExpansionConfig = syn::parse_str("exclude_macros = (macro_rules)").unwrap();
        let mut expected = flust(r#"macro_rules! make_string { @@"untouched" }"#);
        expected.extend(quote!(::std::string::String::from("rewritten")));

        assert_eq!(
            config.rewrite(input, &literal_string_rewrite).to_string(),
            expected.to_string()
        );
    }

    /// Preserves the complete input group of an excluded macro invocation.
    #[test]
    fn leaves_excluded_macro_invocation_opaque() {
        let input = flust(r#"anything!(@@"macro input")"#);
        let original = input.to_string();
        let config: ExpansionConfig = syn::parse_str("exclude_macros = (anything)").unwrap();

        assert_eq!(
            config.rewrite(input, &literal_string_rewrite).to_string(),
            original
        );
    }

    /// Recurses into an unlisted macro invocation.
    #[test]
    fn descends_into_unlisted_macro_invocation() {
        let input = flust(r#"anything!(@@"macro input")"#);

        assert!(rewrite_all(input).to_string().contains("String :: from"));
    }

    /// Does not confuse an ordinary negation token with a macro invocation.
    #[test]
    fn descends_into_negated_condition_groups() {
        let input = flust(r#"if !(@@"condition")"#);

        assert!(rewrite_all(input).to_string().contains("String :: from"));
    }

    /// Consumes the private configuration envelope before transforming input.
    #[test]
    fn removes_shared_config_before_rewriting() {
        let input = flust(
            r#"#![__these_macros_should_be_illegal_config(exclude_macros = (anything))]
               anything!(@@"untouched") @@"rewritten""#,
        );
        let output = expand(input).unwrap().to_string();
        let mut expected = flust(r#"anything!(@@"untouched")"#);
        expected.extend(quote!(::std::string::String::from("rewritten")));

        assert_eq!(output, expected.to_string());
        assert!(!output.contains("__these_macros_should_be_illegal_config"));
    }

    /// Leaves character literals and lifetimes unchanged.
    #[test]
    fn leaves_character_literals_and_lifetimes_alone() {
        let input = quote!('"' 'lifetime);
        let original = input.to_string();

        assert_eq!(rewrite_all(input).to_string(), original);
    }
}
