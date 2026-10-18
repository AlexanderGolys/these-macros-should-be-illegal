//! Shared configuration and recursive token traversal for syntax-rewriting macros.

use proc_macro2::{Delimiter, Group, Spacing, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Error, Ident, ItemMacro, Token,
    meta::ParseNestedMeta,
    parse::{Parse, ParseStream, Parser},
    parse2,
    punctuated::Punctuated,
};

use super::arguments::reject_macro_call;

use Delimiter::Bracket;
use TokenTree::{Group as GroupTT, Ident as IdentTT, Punct as PunctTT};

/// Option naming macros whose invocation inputs stay opaque: `exclude_macros = (a, b)`.
const EXCLUDE_MACROS_OPTION: &str = "exclude_macros";

/// Macro names whose invocation inputs must remain opaque during preprocessing.
#[derive(Clone, Default)]
struct ExcludedMacros(
    /// Exact, possibly raw, macro identifiers to skip.
    Vec<Ident>,
);

/// Parses excluded macro names from shared configuration syntax.
impl Parse for ExcludedMacros {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let names = Punctuated::<Ident, Token![,]>::parse_terminated_with(input, |input| {
            reject_macro_call(input, "a macro name")?;
            input.parse()
        })?;
        Ok(Self(names.into_iter().collect()))
    }
}

/// Shared options carried through a chain of syntax-rewriting macros.
#[derive(Clone, Default)]
pub(crate) struct ExpansionConfig {
    /// Macro invocations excluded from recursive rewriting.
    excluded_macros: ExcludedMacros,
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

/// Provides serialization and recursive traversal operations.
impl ExpansionConfig {
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

    /// Prefixes a macro input with the options spelling this configuration.
    pub(crate) fn configure_input(&self, input: TokenStream) -> TokenStream {
        if self.is_empty() {
            return input;
        }

        let option = format_ident!("{EXCLUDE_MACROS_OPTION}");
        let excluded = &self.excluded_macros.0;
        quote! {
            #option = (#(#excluded),*);
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

/// Removes and parses the leading options of a rewriting macro's input, when present.
///
/// The options are written as the arguments of [`expand!`](crate::expand) are,
/// `exclude_macros = (name, ...);`, and end at the first semicolon.
pub(crate) fn split_option_prefix(
    input: TokenStream,
) -> syn::Result<(ExpansionConfig, TokenStream)> {
    let tokens: Vec<_> = input.into_iter().collect();
    if !starts_with_option(&tokens) {
        return Ok((ExpansionConfig::default(), tokens.into_iter().collect()));
    }

    let end = tokens
        .iter()
        .position(|token| is_punctuation(token, ';'))
        .ok_or_else(|| {
            Error::new(
                tokens[0].span(),
                format!("expected `;` after the `{EXCLUDE_MACROS_OPTION}` option"),
            )
        })?;
    let options = tokens[..end].iter().cloned().collect();

    Ok((
        parse2(options)?,
        tokens[end + 1..].iter().cloned().collect(),
    ))
}

/// Reports whether a token prefix spells one of the shared named options.
fn starts_with_option(tokens: &[TokenTree]) -> bool {
    matches!(tokens.first(), Some(IdentTT(name)) if name == EXCLUDE_MACROS_OPTION)
        && matches!(tokens.get(1), Some(token) if is_punctuation(token, '='))
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

    /// A leading option prefix is consumed and the remaining stream left untouched.
    #[test]
    fn splits_the_leading_option_prefix() {
        let input: TokenStream = "exclude_macros = (first, second); fn body() {}"
            .parse()
            .unwrap();
        let (config, rest) = split_option_prefix(input).unwrap();

        assert!(config.is_excluded(&syn::parse_quote!(second)));
        assert_eq!(
            rest.to_string(),
            quote!(
                fn body() {}
            )
            .to_string()
        );
    }

    /// Input that does not begin with an option is returned in full.
    #[test]
    fn keeps_input_without_an_option_prefix() {
        let input: TokenStream = "exclude_macros; fn body() {}".parse().unwrap();
        let original = input.to_string();
        let (config, rest) = split_option_prefix(input).unwrap();

        assert!(config.is_empty());
        assert_eq!(rest.to_string(), original);
    }

    /// An option prefix without its terminating semicolon is rejected.
    #[test]
    fn rejects_unterminated_options() {
        let input: TokenStream = "exclude_macros = (first)".parse().unwrap();
        let error = split_option_prefix(input)
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();

        assert!(error.contains("expected `;`"));
    }

    /// A configured input is read back as the configuration that wrote it.
    #[test]
    fn configured_input_round_trips() {
        let config: ExpansionConfig = syn::parse_str("exclude_macros = (first)").unwrap();
        let input = config.configure_input(quote!(
            fn body() {}
        ));
        let (restored, rest) = split_option_prefix(input).unwrap();

        assert!(restored.is_excluded(&syn::parse_quote!(first)));
        assert_eq!(
            rest.to_string(),
            quote!(
                fn body() {}
            )
            .to_string()
        );
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
