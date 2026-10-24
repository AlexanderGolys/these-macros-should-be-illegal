//! Shared configuration and recursive token traversal for syntax-rewriting macros.

use crate::helpers::arguments::rewriting::Arguments as ExpansionConfig;

use proc_macro2::{Delimiter, Group, Spacing, TokenStream, TokenTree};
use syn::{ItemMacro, parse2};

use Delimiter::Bracket;
use TokenTree::{Group as GroupTT, Ident as IdentTT, Punct as PunctTT};

/// Provides serialization and recursive traversal operations.
impl ExpansionConfig {
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

    /// Returns the length of an attribute or excluded macro prefix that must stay opaque.
    fn opaque_prefix_length(&self, tokens: &[TokenTree]) -> Option<usize> {
        if let Some(length) = super::arguments::input::opaque_prefix_length(tokens) {
            return Some(length);
        }
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
