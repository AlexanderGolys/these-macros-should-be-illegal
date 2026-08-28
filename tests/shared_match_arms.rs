//! Consumer tests for shared match-arm right-hand sides.

use these_macros_should_be_illegal::{excluded_macros, shared_match_arms};

macro_rules! raw_match_syntax {
    ({ A || B => rhs }) => {
        "opaque"
    };
}

#[excluded_macros(raw_match_syntax)]
shared_match_arms! {
    /// Invokes a macro whose input must remain untouched.
    fn excluded_macro_input() -> &'static str {
        raw_match_syntax!({ A || B => rhs })
    }
}

these_macros_should_be_illegal::expand!(
    these_macros_should_be_illegal::shared_match_arms;
    #[path = "fixtures/shared_match_arms_module.rs"]
    mod shared_match_arms_module;
);

/// First payload type used to require independently typed pattern bindings.
struct Group {
    /// Text retained after the token.
    trailing: Vec<&'static str>,
}

/// Second, deliberately distinct payload type.
struct Literal {
    /// Text retained after the token.
    trailing: Vec<&'static str>,
}

/// Shared operation implemented by both distinct payload types.
trait Token {
    /// Returns the text trailing this token.
    fn trailing_trivia(&self) -> &[&'static str];
}

/// Implements the shared trailing-trivia operation.
impl Token for Group {
    /// Borrows the group's trailing trivia.
    fn trailing_trivia(&self) -> &[&'static str] {
        &self.trailing
    }
}

/// Implements the shared trailing-trivia operation.
impl Token for Literal {
    /// Borrows the literal's trailing trivia.
    fn trailing_trivia(&self) -> &[&'static str] {
        &self.trailing
    }
}

/// Sum whose variants cannot participate in a native Rust or-pattern binding.
enum TokenTree {
    /// Group payload.
    Group(Group),
    /// Literal payload.
    Literal(Literal),
}

/// Four variants grouped into two ordinary or-patterns with distinct payload types.
enum NativeOrPattern {
    /// First numeric variant.
    A(u32),
    /// Second numeric variant.
    B(u32),
    /// First character variant.
    C(char),
    /// Second character variant.
    D(char),
}

/// Outer choice used to verify bottom-up rewriting of nested matches.
enum Outer {
    /// First route to an inner token.
    First(TokenTree),
    /// Second route to an inner token.
    Second(TokenTree),
}

/// Choice used to prove that an arm attribute reaches every generated arm.
enum Configured {
    /// First conditionally removed arm.
    A,
    /// Second conditionally removed arm.
    B,
}

shared_match_arms! {
    /// Returns trailing trivia through independently typed generated match arms.
    fn trailing_trivia(token: &TokenTree) -> &[&'static str] {
        match token {
            TokenTree::Group(token) || TokenTree::Literal(token) => token.trailing_trivia(),
        }
    }

    /// Applies a distinct guard to each alternative before sharing its RHS.
    fn has_selected_length(token: &TokenTree) -> bool {
        match token {
            (TokenTree::Group(token) if token.trailing_trivia().len() == 1)
                || (TokenTree::Literal(token) if token.trailing_trivia().len() == 2)
                => true,
            _ => false,
        }
    }

    /// Leaves an ordinary logical OR in an ordinary Rust guard untouched.
    fn guarded_trivia_len(token: &TokenTree) -> usize {
        match token {
            TokenTree::Group(token)
                if token.trailing_trivia().is_empty()
                    || !token.trailing_trivia().is_empty()
                => token.trailing_trivia().len(),
            _ => 0,
        }
    }

    /// Preserves native or-patterns within each independently typed alternative.
    fn native_or_pattern_text(value: NativeOrPattern) -> String {
        match value {
            NativeOrPattern::A(value) | NativeOrPattern::B(value)
                || NativeOrPattern::C(value) | NativeOrPattern::D(value) => value.to_string(),
        }
    }

    /// Rewrites an inner shared match before cloning its containing outer arm.
    fn nested_text(value: Outer) -> String {
        match value {
            Outer::First(token) || Outer::Second(token) => match token {
                TokenTree::Group(token) || TokenTree::Literal(token) =>
                    token.trailing_trivia().len().to_string(),
            },
        }
    }

    /// Copies arm attributes so all generated alternatives have the same condition.
    fn configured_arm(value: Configured) -> bool {
        match value {
            #[cfg(any())]
            Configured::A || Configured::B => false,
            _ => true,
        }
    }

    /// Leaves logical OR outside a match arm completely unchanged.
    fn ordinary_logical_or(left: bool, right: bool) -> bool {
        left || right
    }
}

/// Shares one operation across bindings with different concrete types.
#[test]
fn duplicates_rhs_for_distinct_payload_types() {
    let group = TokenTree::Group(Group {
        trailing: vec!["group"],
    });
    let literal = TokenTree::Literal(Literal {
        trailing: vec!["first", "second"],
    });

    assert_eq!(trailing_trivia(&group), ["group"]);
    assert_eq!(trailing_trivia(&literal), ["first", "second"]);
}

/// Supports separately guarded synthetic alternatives.
#[test]
fn supports_component_guards() {
    let group = TokenTree::Group(Group {
        trailing: vec!["group"],
    });
    let literal = TokenTree::Literal(Literal {
        trailing: vec!["first", "second"],
    });

    assert!(has_selected_length(&group));
    assert!(has_selected_length(&literal));
    assert_eq!(guarded_trivia_len(&group), 1);
}

/// Preserves native or-pattern semantics within each shared alternative.
#[test]
fn supports_native_or_patterns() {
    assert_eq!(native_or_pattern_text(NativeOrPattern::A(7)), "7");
    assert_eq!(native_or_pattern_text(NativeOrPattern::B(8)), "8");
    assert_eq!(native_or_pattern_text(NativeOrPattern::C('x')), "x");
    assert_eq!(native_or_pattern_text(NativeOrPattern::D('y')), "y");
}

/// Rewrites nested shared matches in every cloned outer RHS.
#[test]
fn supports_nested_matches() {
    let group = TokenTree::Group(Group {
        trailing: vec!["group"],
    });
    let literal = TokenTree::Literal(Literal {
        trailing: vec!["first", "second"],
    });

    assert_eq!(nested_text(Outer::First(group)), "1");
    assert_eq!(nested_text(Outer::Second(literal)), "2");
}

/// Duplicates arm attributes and ignores unrelated logical OR expressions.
#[test]
fn preserves_arm_attributes_and_unrelated_syntax() {
    assert!(configured_arm(Configured::A));
    assert!(configured_arm(Configured::B));
    assert!(ordinary_logical_or(true, false));
    assert!(ordinary_logical_or(false, true));
    assert!(!ordinary_logical_or(false, false));
}

/// Composes with the shared exclusion envelope and external-module expander.
#[test]
fn composes_with_other_flust_macros() {
    assert_eq!(excluded_macro_input(), "opaque");
    assert_eq!(shared_match_arms_module::describe_number(7), "7");
    assert_eq!(shared_match_arms_module::describe_character('x'), "x");
}
