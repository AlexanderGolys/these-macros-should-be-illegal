//! Implementation of shared match-arm right-hand sides separated by `||`.

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
    Arm, Attribute, Error, Expr, Pat, Token, parenthesized,
    parse::{Parse, ParseStream, discouraged::Speculative},
    parse2,
};

#[cfg(test)]
use crate::helpers::preprocessing::ExpansionConfig;
use crate::helpers::preprocessing::split_config_prefix;

use TokenTree::Group as GroupTT;

/// One pattern and optional guard on the left of a shared arm.
struct ArmAlternative {
    /// Pattern parsed by Syn's ordinary match-pattern parser.
    pat: Pat,
    /// Optional guard belonging only to this alternative.
    guard: Option<(Token![if], Box<Expr>)>,
}

/// A Syn match arm extended with additional independently typed alternatives.
struct SharedArm {
    /// Complete ordinary arm holding the first LHS and shared RHS.
    arm: Arm,
    /// Additional LHS nodes separated from the first by `||`.
    alternatives: Vec<ArmAlternative>,
}

/// Every arm contained directly in one candidate match-body group.
struct ArmList {
    /// Parsed ordinary and extended arms in source order.
    arms: Vec<SharedArm>,
    /// Whether at least one arm used the extended separator.
    has_shared_arm: bool,
}

/// One Syn arm parsed from a reconstructed prefix, plus following group tokens.
struct ArmPrefix {
    /// Complete arm parsed using Syn's own implementation.
    arm: Arm,
    /// Tokens following that arm in the same match body.
    remaining: TokenStream,
}

/// Parses one LHS using Syn, with a parenthesized local guard as the sole extension.
impl Parse for ArmAlternative {
    /// Parses either an ordinary pattern or `(pattern if guard)`.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(syn::token::Paren) {
            let fork = input.fork();
            let content;
            parenthesized!(content in fork);
            if let Ok(pat) = content.call(Pat::parse_multi_with_leading_vert)
                && content.peek(Token![if])
            {
                let if_token = content.parse()?;
                let guard = content.parse()?;
                if !content.is_empty() {
                    return Err(content.error("unexpected tokens after alternative guard"));
                }
                input.advance_to(&fork);
                return Ok(Self {
                    pat,
                    guard: Some((if_token, Box::new(guard))),
                });
            }
        }

        let pat = input.call(Pat::parse_multi_with_leading_vert)?;
        let guard = if input.peek(Token![if]) {
            Some((input.parse()?, Box::new(input.parse()?)))
        } else {
            None
        };
        Ok(Self { pat, guard })
    }
}

/// Parses the extended LHS before delegating the complete ordinary arm to Syn.
impl Parse for SharedArm {
    /// Parses attributes, alternatives, and the shared Syn arm suffix.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let first: ArmAlternative = input.parse()?;
        let mut alternatives = Vec::new();
        while input.peek(Token![||]) {
            input.parse::<Token![||]>()?;
            alternatives.push(input.parse()?);
        }

        let arm = parse_syn_arm(input, attrs, first)?;
        Ok(Self { arm, alternatives })
    }
}

/// Parses every arm until the surrounding brace group is exhausted.
impl Parse for ArmList {
    /// Parses ordinary and shared arms using the same custom arm node.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut arms = Vec::new();
        let mut has_shared_arm = false;
        while !input.is_empty() {
            let arm: SharedArm = input.parse()?;
            has_shared_arm |= !arm.alternatives.is_empty();
            arms.push(arm);
        }
        Ok(Self {
            arms,
            has_shared_arm,
        })
    }
}

/// Parses one ordinary Syn arm and retains all following tokens.
impl Parse for ArmPrefix {
    /// Delegates the arm grammar completely to `syn::Arm`.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            arm: input.parse()?,
            remaining: input.parse()?,
        })
    }
}

/// Rewrites shared match-arm alternatives throughout a procedural macro input.
pub(crate) fn shared_match_arms(input: TokenStream) -> TokenStream {
    expand(input).unwrap_or_else(Error::into_compile_error)
}

/// Removes shared configuration and rewrites candidate arm groups bottom-up.
fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let (config, input) = split_config_prefix(input)?;
    Ok(config.rewrite_bottom_up(input, &shared_arm_group_rewrite))
}

/// Reconstructs an ordinary first arm and lets Syn parse its complete suffix.
fn parse_syn_arm(
    input: ParseStream,
    attrs: Vec<Attribute>,
    first: ArmAlternative,
) -> syn::Result<Arm> {
    let fork = input.fork();
    let remaining: TokenStream = fork.parse()?;
    let token_count = remaining.clone().into_iter().count();
    let pat = &first.pat;
    let guard = first
        .guard
        .as_ref()
        .map(|(if_token, guard)| quote!(#if_token #guard));
    let parsed = parse2::<ArmPrefix>(quote! {
        #(#attrs)* #pat #guard #remaining
    })?;
    let consumed = token_count - parsed.remaining.into_iter().count();

    for _ in 0..consumed {
        input.parse::<TokenTree>()?;
    }

    Ok(parsed.arm)
}

/// Replaces one brace group when its complete contents parse as extended arms.
fn shared_arm_group_rewrite(tokens: &[TokenTree]) -> Option<(usize, TokenStream)> {
    let GroupTT(group) = tokens.first()? else {
        return None;
    };
    if group.delimiter() != Delimiter::Brace {
        return None;
    }

    let arms = parse2::<ArmList>(group.stream()).ok()?;
    if !arms.has_shared_arm {
        return None;
    }

    let mut rewritten = Group::new(Delimiter::Brace, lower_arms(arms.arms));
    rewritten.set_span(group.span());
    Some((1, GroupTT(rewritten).into()))
}

/// Expands every additional LHS into an independent ordinary Syn arm.
fn lower_arms(arms: Vec<SharedArm>) -> TokenStream {
    let mut output = TokenStream::new();

    for SharedArm {
        mut arm,
        alternatives,
    } in arms
    {
        if alternatives.is_empty() {
            arm.to_tokens(&mut output);
            continue;
        }

        arm.comma.get_or_insert_with(Default::default);
        arm.to_tokens(&mut output);
        for alternative in alternatives {
            Arm {
                attrs: arm.attrs.clone(),
                pat: alternative.pat,
                guard: alternative.guard,
                fat_arrow_token: arm.fat_arrow_token,
                body: arm.body.clone(),
                comma: arm.comma,
            }
            .to_tokens(&mut output);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    //! Unit tests for shared match-arm rewriting.

    use super::*;

    /// Parses deliberately extended arm syntax as an unrestricted token stream.
    fn flust(input: &str) -> TokenStream {
        input.parse().unwrap()
    }

    /// Applies the transform with no excluded macro names.
    fn rewrite(input: &str) -> String {
        ExpansionConfig::default()
            .rewrite_bottom_up(flust(input), &shared_arm_group_rewrite)
            .to_string()
    }

    /// Clones one RHS into independently typed ordinary match arms.
    #[test]
    fn duplicates_shared_right_hand_sides() {
        let output =
            rewrite("match value { E::A(value) || E::B(value) || E::C(value) => value.text(), }");

        for pattern in ["E :: A (value)", "E :: B (value)", "E :: C (value)"] {
            assert!(output.contains(&format!("{pattern} => value . text ()")));
        }
        assert!(!output.contains("||"));
    }

    /// Unwraps a separately guarded parenthesized component.
    #[test]
    fn supports_guards_per_alternative() {
        let output = rewrite(
            "match value { (E::A(value) if value.ready() || value.forced()) || (E::B(value) if value.valid()) => true, _ => false }",
        );

        assert!(output.contains("E :: A (value) if value . ready () || value . forced () => true"));
        assert!(output.contains("E :: B (value) if value . valid () => true"));
    }

    /// Leaves Rust's ordinary logical OR inside an arm guard unchanged.
    #[test]
    fn preserves_logical_or_in_ordinary_guards() {
        let input =
            "match value { E::A(value) if value.ready() || value.forced() => true, _ => false }";

        assert_eq!(rewrite(input), flust(input).to_string());
    }

    /// Uses Syn's ordinary or-pattern parsing inside each extended alternative.
    #[test]
    fn preserves_native_or_patterns() {
        let output = rewrite("match value { A | B || C | D => rhs, }");

        assert!(output.contains("A | B => rhs"));
        assert!(output.contains("C | D => rhs"));
    }

    /// Rewrites nested shared matches before parsing their containing arm.
    #[test]
    fn rewrites_nested_shared_arms_bottom_up() {
        let output =
            rewrite("match outer { A || B => match inner { C(value) || D(value) => value, }, }");

        assert!(!output.contains("||"));
        assert_eq!(output.matches("C (value) => value").count(), 2);
        assert_eq!(output.matches("D (value) => value").count(), 2);
    }

    /// Leaves attribute payloads opaque while rewriting adjacent ordinary groups.
    #[test]
    fn leaves_attribute_contents_opaque() {
        let output =
            rewrite("#[opaque(match value { A || B => rhs })] match value { A || B => rhs, }");

        assert!(output.contains("opaque (match value { A || B => rhs })"));
        assert_eq!(output.matches("||").count(), 1);
        assert!(output.contains("A => rhs"));
        assert!(output.contains("B => rhs"));
    }

    /// Reuses Syn's boundary rules for consecutive block arms without commas.
    #[test]
    fn preserves_comma_optional_block_arms() {
        let output = rewrite("match value { A || B => { first() } C || D => { second() } }");

        assert!(output.contains("A => { first () }"));
        assert!(output.contains("B => { first () }"));
        assert!(output.contains("C => { second () }"));
        assert!(output.contains("D => { second () }"));
    }

    /// Copies an arm attribute to every generated ordinary arm.
    #[test]
    fn duplicates_arm_attributes_without_rewriting_them() {
        let output = rewrite("match value { #[cfg(any())] A || B => rhs, C => other }");

        assert_eq!(output.matches("# [cfg (any ())]").count(), 2);
        assert!(output.contains("# [cfg (any ())] A => rhs"));
        assert!(output.contains("# [cfg (any ())] B => rhs"));
    }
}
