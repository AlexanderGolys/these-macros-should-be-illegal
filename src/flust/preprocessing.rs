//! Shared configuration and recursive token traversal for syntax-rewriting macros.

use proc_macro2::{Delimiter, Group, Spacing, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::{
    Error, Ident, ItemMacro, Meta, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse2,
    punctuated::Punctuated,
};

use Delimiter::Bracket;
use TokenTree::{Group as GroupTT, Ident as IdentTT, Punct as PunctTT};

/// The private inner attribute used to pass preprocessing configuration between macros.
const CONFIG_ATTRIBUTE: &str = "__these_macros_should_be_illegal_config";

/// Macro names whose invocation inputs must remain opaque during preprocessing.
#[derive(Clone, Default)]
pub(super) struct ExcludedMacros(
    /// Exact, possibly raw, macro identifiers to skip.
    Vec<Ident>,
);

/// Shared options carried through a chain of syntax-rewriting macros.
#[derive(Clone, Default)]
pub(super) struct ExpansionConfig {
    /// Macro invocations excluded from recursive rewriting.
    excluded_macros: ExcludedMacros,
}

/// Parses excluded macro names from shared configuration syntax.
impl Parse for ExcludedMacros {
    /// Parses a comma-separated list of macro identifiers.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(
            <Punctuated<Ident, Token![,]>>::parse_terminated(input)?
                .into_iter()
                .collect(),
        ))
    }
}

/// Parses the complete shared preprocessing configuration.
impl Parse for ExpansionConfig {
    /// Parses the shared named preprocessing options.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut config = Self::default();
        let mut has_excluded_macros = false;

        while !input.is_empty() {
            parse_config_option(input, &mut config, &mut has_excluded_macros)?;

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(config)
    }
}

/// Provides construction, merging, serialization, and recursive traversal operations.
impl ExpansionConfig {
    /// Creates a configuration containing the supplied excluded macro names.
    pub(super) fn excluding(excluded_macros: ExcludedMacros) -> Self {
        Self { excluded_macros }
    }

    /// Reports whether a macro identifier is excluded exactly, including rawness.
    pub(super) fn is_excluded(&self, identifier: &Ident) -> bool {
        self.excluded_macros
            .0
            .iter()
            .any(|excluded| excluded == identifier)
    }

    /// Reports whether the configuration carries no options.
    fn is_empty(&self) -> bool {
        self.excluded_macros.0.is_empty()
    }

    /// Adds options from another envelope without duplicating macro identifiers.
    pub(super) fn merge(&mut self, other: Self) {
        for identifier in other.excluded_macros.0 {
            if !self.is_excluded(&identifier) {
                self.excluded_macros.0.push(identifier);
            }
        }
    }

    /// Recursively applies a token transform while preserving opaque token regions.
    pub(super) fn rewrite<F>(&self, input: TokenStream, transform: &F) -> TokenStream
    where
        F: Fn(&[TokenTree]) -> Option<(usize, TokenStream)>,
    {
        let tokens: Vec<_> = input.into_iter().collect();
        let mut output = TokenStream::new();
        let mut index = 0;

        while index < tokens.len() {
            if let Some(length) = self.opaque_prefix_length(&tokens[index..]) {
                output.extend(tokens[index..index + length].iter().cloned());
                index += length;
                continue;
            }

            if let Some((length, replacement)) = transform(&tokens[index..]) {
                output.extend(replacement);
                index += length;
                continue;
            }

            match tokens[index].clone() {
                GroupTT(group) => output.extend([GroupTT(self.rewrite_group(group, transform))]),
                token => output.extend([token]),
            }
            index += 1;
        }

        output
    }

    /// Prefixes a macro input with this configuration's private inner attribute.
    pub(super) fn configure_input(&self, input: TokenStream) -> TokenStream {
        if self.is_empty() {
            return input;
        }

        let attribute = format_ident!("{CONFIG_ATTRIBUTE}");
        let excluded = &self.excluded_macros.0;
        quote! {
            #![#attribute(exclude_macros = (#(#excluded),*))]
            #input
        }
    }

    /// Returns the length of an attribute or excluded macro prefix that must stay opaque.
    fn opaque_prefix_length(&self, tokens: &[TokenTree]) -> Option<usize> {
        if matches!(tokens.first(), Some(token) if is_punctuation(token, '#')) {
            if matches!(tokens.get(1), Some(GroupTT(group)) if group.delimiter() == Bracket) {
                return Some(2);
            }

            if matches!(tokens.get(1), Some(token) if is_punctuation(token, '!'))
                && matches!(tokens.get(2), Some(GroupTT(group)) if group.delimiter() == Bracket)
            {
                return Some(3);
            }
        }

        if let Some(IdentTT(identifier)) = tokens.first()
            && self.is_excluded(identifier)
            && matches!(tokens.get(1), Some(token) if is_punctuation(token, '!'))
        {
            if matches!(tokens.get(2), Some(GroupTT(_))) {
                return Some(3);
            }
            if identifier == "macro_rules" {
                return declarative_macro_prefix_length(tokens);
            }
        }

        None
    }

    /// Rebuilds a group after recursively rewriting its stream and preserving its span.
    fn rewrite_group<F>(&self, group: Group, transform: &F) -> Group
    where
        F: Fn(&[TokenTree]) -> Option<(usize, TokenStream)>,
    {
        let mut rewritten = Group::new(group.delimiter(), self.rewrite(group.stream(), transform));
        rewritten.set_span(group.span());
        rewritten
    }
}

/// Parses one named option into a shared preprocessing configuration.
pub(super) fn parse_config_option(
    input: ParseStream,
    config: &mut ExpansionConfig,
    has_excluded_macros: &mut bool,
) -> syn::Result<()> {
    let name = Ident::parse_any(input)?;
    input.parse::<Token![=]>()?;

    if name != "exclude_macros" {
        return Err(Error::new(
            name.span(),
            "unknown macro configuration option",
        ));
    }
    if *has_excluded_macros {
        return Err(Error::new(name.span(), "duplicate `exclude_macros` option"));
    }

    let content;
    syn::parenthesized!(content in input);
    config.excluded_macros = content.parse()?;
    *has_excluded_macros = true;

    Ok(())
}

/// Removes and parses a leading private configuration attribute, when present.
pub(super) fn split_config_prefix(
    input: TokenStream,
) -> syn::Result<(ExpansionConfig, TokenStream)> {
    let tokens: Vec<_> = input.into_iter().collect();

    let Some(PunctTT(hash)) = tokens.first() else {
        return Ok((ExpansionConfig::default(), tokens.into_iter().collect()));
    };
    let Some(PunctTT(bang)) = tokens.get(1) else {
        return Ok((ExpansionConfig::default(), tokens.into_iter().collect()));
    };
    let Some(GroupTT(group)) = tokens.get(2) else {
        return Ok((ExpansionConfig::default(), tokens.into_iter().collect()));
    };

    if hash.as_char() != '#' || bang.as_char() != '!' || group.delimiter() != Bracket {
        return Ok((ExpansionConfig::default(), tokens.into_iter().collect()));
    }
    if !matches!(group.stream().into_iter().next(), Some(IdentTT(identifier)) if identifier == CONFIG_ATTRIBUTE)
    {
        return Ok((ExpansionConfig::default(), tokens.into_iter().collect()));
    }

    let meta = parse2::<Meta>(group.stream())?;
    let Meta::List(meta) = meta else {
        return Err(Error::new(
            group.span(),
            "expected macro configuration arguments",
        ));
    };
    let config = meta.parse_args::<ExpansionConfig>()?;
    let rest = tokens.into_iter().skip(3).collect();

    Ok((config, rest))
}

/// Reports whether a token is punctuation with the requested character.
pub(super) fn is_punctuation(token: &TokenTree, expected: char) -> bool {
    matches!(token, PunctTT(punctuation) if punctuation.as_char() == expected)
}

/// Reports whether a token is joint punctuation with the requested character.
pub(super) fn is_joint_punctuation(token: &TokenTree, expected: char) -> bool {
    matches!(token, PunctTT(punctuation) if punctuation.as_char() == expected && punctuation.spacing() == Spacing::Joint)
}

/// Finds the complete length of a declarative macro definition at the token prefix.
fn declarative_macro_prefix_length(tokens: &[TokenTree]) -> Option<usize> {
    let GroupTT(body) = tokens.get(3)? else {
        return None;
    };
    let length = match body.delimiter() {
        Delimiter::Brace => 4,
        _ if matches!(tokens.get(4), Some(token) if is_punctuation(token, ';')) => 5,
        _ => return None,
    };
    let item: TokenStream = tokens[..length].iter().cloned().collect();
    let item = parse2::<ItemMacro>(item).ok()?;

    (item.ident.is_some() && item.mac.path.is_ident("macro_rules")).then_some(length)
}

#[cfg(test)]
mod tests {
    //! Unit tests for shared preprocessing configuration.

    use super::*;

    /// Parses and compares excluded macro identifiers.
    #[test]
    fn parses_excluded_macro_names() {
        let config: ExpansionConfig = syn::parse_str("exclude_macros = (first, second,)").unwrap();
        let first = syn::parse_quote!(first);
        let second = syn::parse_quote!(second);
        let other = syn::parse_quote!(other);

        assert!(config.is_excluded(&first));
        assert!(config.is_excluded(&second));
        assert!(!config.is_excluded(&other));
    }

    /// Keeps raw identifiers distinct while matching exact raw macro names.
    #[test]
    fn excludes_exact_raw_macro_name() {
        let input: TokenStream = r#"r#if!(@@"macro input")"#.parse().unwrap();
        let original = input.to_string();
        let config: ExpansionConfig = syn::parse_str("exclude_macros = (r#if)").unwrap();

        assert_eq!(config.rewrite(input, &|_| None).to_string(), original);
    }
}
