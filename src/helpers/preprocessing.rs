//! Shared configuration and recursive token traversal for syntax-rewriting macros.

use proc_macro2::{Delimiter, Group, Spacing, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Ident, ItemMacro, Token,
    meta::ParseNestedMeta,
    parse::{Parse, ParseStream, Parser},
    parse2,
    punctuated::Punctuated,
};

use super::arguments::reject_macro_call;

use Delimiter::Bracket;
use TokenTree::{Group as GroupTT, Ident as IdentTT, Punct as PunctTT};

/// The private inner attribute used to pass preprocessing configuration between macros.
const CONFIG_ATTRIBUTE: &str = "__these_macros_should_be_illegal_config";

/// Option naming macros whose invocation inputs stay opaque: `exclude_macros = (a, b)`.
const EXCLUDE_MACROS_OPTION: &str = "exclude_macros";

/// Macro names whose invocation inputs must remain opaque during preprocessing.
#[derive(Clone, Default)]
pub(crate) struct ExcludedMacros(
    /// Exact, possibly raw, macro identifiers to skip.
    Vec<Ident>,
);

/// Shared options carried through a chain of syntax-rewriting macros.
#[derive(Clone, Default)]
pub(crate) struct ExpansionConfig {
    /// Macro invocations excluded from recursive rewriting.
    excluded_macros: ExcludedMacros,
}

/// Parses excluded macro names from shared configuration syntax.
impl Parse for ExcludedMacros {
    /// Parses a comma-separated list of macro identifiers.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let names = Punctuated::<Ident, Token![,]>::parse_terminated_with(input, |input| {
            reject_macro_call(input, "a macro name")?;
            input.parse()
        })?;
        Ok(Self(names.into_iter().collect()))
    }
}

/// Parses the complete shared preprocessing configuration.
impl Parse for ExpansionConfig {
    /// Parses the shared named preprocessing options.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut config = Self::default();
        let mut has_excluded_macros = false;
        syn::meta::parser(|option| {
            parse_config_option(option, &mut config, &mut has_excluded_macros)
        })
        .parse2(input.parse()?)?;
        Ok(config)
    }
}

/// Provides construction, merging, serialization, and recursive traversal operations.
impl ExpansionConfig {
    /// Creates a configuration containing the supplied excluded macro names.
    pub(crate) fn excluding(excluded_macros: ExcludedMacros) -> Self {
        Self { excluded_macros }
    }

    /// Reports whether a macro identifier is excluded exactly, including rawness.
    pub(crate) fn is_excluded(&self, identifier: &Ident) -> bool {
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
    pub(crate) fn merge(&mut self, other: Self) {
        for identifier in other.excluded_macros.0 {
            if !self.is_excluded(&identifier) {
                self.excluded_macros.0.push(identifier);
            }
        }
    }

    /// Recursively applies a token transform while preserving opaque token regions.
    pub(crate) fn rewrite<F>(&self, input: TokenStream, transform: &F) -> TokenStream
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

    /// Recursively transforms nested groups before transforming their containing stream.
    pub(crate) fn rewrite_bottom_up<F>(&self, input: TokenStream, transform: &F) -> TokenStream
    where
        F: Fn(&[TokenTree]) -> Option<(usize, TokenStream)>,
    {
        let tokens: Vec<_> = input.into_iter().collect();
        let mut prepared = TokenStream::new();
        let mut index = 0;

        while index < tokens.len() {
            if let Some(length) = self.opaque_prefix_length(&tokens[index..]) {
                prepared.extend(tokens[index..index + length].iter().cloned());
                index += length;
                continue;
            }

            match tokens[index].clone() {
                GroupTT(group) => {
                    prepared.extend([GroupTT(self.rewrite_bottom_up_group(group, transform))]);
                }
                token => prepared.extend([token]),
            }
            index += 1;
        }

        let tokens: Vec<_> = prepared.into_iter().collect();
        let mut output = TokenStream::new();
        let mut index = 0;
        while index < tokens.len() {
            if let Some(length) = self.opaque_prefix_length(&tokens[index..]) {
                output.extend(tokens[index..index + length].iter().cloned());
                index += length;
            } else if let Some((length, replacement)) = transform(&tokens[index..]) {
                output.extend(replacement);
                index += length;
            } else {
                output.extend([tokens[index].clone()]);
                index += 1;
            }
        }

        output
    }

    /// Prefixes a macro input with this configuration's private inner attribute.
    pub(crate) fn configure_input(&self, input: TokenStream) -> TokenStream {
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

    /// Rebuilds a group after bottom-up recursive rewriting and preserves its span.
    fn rewrite_bottom_up_group<F>(&self, group: Group, transform: &F) -> Group
    where
        F: Fn(&[TokenTree]) -> Option<(usize, TokenStream)>,
    {
        let mut rewritten = Group::new(
            group.delimiter(),
            self.rewrite_bottom_up(group.stream(), transform),
        );
        rewritten.set_span(group.span());
        rewritten
    }
}

/// Parses one `name = value` option into a shared preprocessing configuration.
pub(crate) fn parse_config_option(
    option: ParseNestedMeta,
    config: &mut ExpansionConfig,
    has_excluded_macros: &mut bool,
) -> syn::Result<()> {
    if !option.path.is_ident(EXCLUDE_MACROS_OPTION) {
        let name = option.path.to_token_stream().to_string().replace(' ', "");
        return Err(option.error(format!(
            "unknown option `{name}`; expected `{EXCLUDE_MACROS_OPTION} = (...)`"
        )));
    }
    if *has_excluded_macros {
        return Err(option.error(format!("duplicate `{EXCLUDE_MACROS_OPTION}` option")));
    }

    let value = option.value()?;
    let content;
    syn::parenthesized!(content in value);
    config.excluded_macros = content.parse()?;
    *has_excluded_macros = true;

    Ok(())
}

/// Removes and parses a leading private configuration attribute, when present.
pub(crate) fn split_config_prefix(
    input: TokenStream,
) -> syn::Result<(ExpansionConfig, TokenStream)> {
    let tokens: Vec<_> = input.into_iter().collect();
    // The private attribute is exactly `#`, `!` and one bracketed group.
    let prefix = tokens.iter().take(3).cloned().collect();
    let Some(attribute) = Attribute::parse_inner
        .parse2(prefix)
        .ok()
        .and_then(|attributes| attributes.into_iter().next())
        .filter(|attribute| attribute.path().is_ident(CONFIG_ATTRIBUTE))
    else {
        return Ok((ExpansionConfig::default(), tokens.into_iter().collect()));
    };

    let mut config = ExpansionConfig::default();
    let mut has_excluded_macros = false;
    attribute.parse_nested_meta(|option| {
        parse_config_option(option, &mut config, &mut has_excluded_macros)
    })?;
    Ok((config, tokens.into_iter().skip(3).collect()))
}

/// Reports whether a token is punctuation with the requested character.
pub(crate) fn is_punctuation(token: &TokenTree, expected: char) -> bool {
    matches!(token, PunctTT(punctuation) if punctuation.as_char() == expected)
}

/// Reports whether a token is joint punctuation with the requested character.
pub(crate) fn is_joint_punctuation(token: &TokenTree, expected: char) -> bool {
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

    /// Unknown, repeated and list-form options are rejected with specific messages.
    #[test]
    fn rejects_malformed_options() {
        let message = |source: &str| {
            syn::parse_str::<ExpansionConfig>(source)
                .err()
                .map(|error| error.to_string())
                .unwrap_or_default()
        };

        assert!(message("exlude_macros = (a)").contains("unknown option `exlude_macros`"));
        assert!(message("exclude_macros = (a), exclude_macros = (b)").contains("duplicate"));
        assert!(message("exclude_macros(a)").contains("expected `=`"));
    }

    /// Trailing commas are accepted both inside the list and after the option.
    #[test]
    fn accepts_trailing_commas() {
        let config: ExpansionConfig = syn::parse_str("exclude_macros = (a, b,),").unwrap();
        assert!(config.is_excluded(&syn::parse_quote!(b)));
    }

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
