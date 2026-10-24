//! Consumer tests for the shared function-like macro parameter envelope.

use these_macros_should_be_illegal::{
    emmun, expand, forward_attributes, lls, macro_args, perm, qf, reflect, sma, stringify_as,
    strutuct,
};

macro_rules! borrowed {
    (@@ $value:literal) => {
        $value
    };
}

#[macro_args(exclude = (borrowed))]
lls! {
    fn configured_strings() -> (String, &'static str) {
        (@@"owned", borrowed!(@@"borrowed"))
    }
}

#[macro_args(prod)]
#[forward_attributes]
#[derive(Debug, PartialEq)]
strutuct! { Packed Pair(u8, u8) }

#[macro_args(product_variants)]
#[forward_attributes]
#[derive(Debug, PartialEq)]
emmun! { AlsoPacked Pair(u8, u8) }

#[macro_args(these_macros_should_be_illegal::lls, exclude = (borrowed))]
expand! {
    #[path = "fixtures/literally_literal_string_module.rs"]
    mod configured_module;
}

macro_rules! receive {
    (@macro_args($name:ident); $value:literal) => {
        const $name: &str = $value;
    };
}

#[macro_args(PAREN)]
receive!("parenthesis");
#[macro_args(BRACKET)]
receive!["bracket"];
#[macro_args(BRACE)]
receive! { "brace" }

/// The attribute also supports cooperating declarative macros and all delimiters.
#[test]
fn forwards_parameters_to_the_invoked_macro() {
    assert_eq!([PAREN, BRACKET, BRACE], ["parenthesis", "bracket", "brace"]);
}

/// Shared exclusions and recursive traversal apply after removing the envelope.
#[test]
fn configures_recursive_rewriting() {
    assert_eq!(configured_strings(), (String::from("owned"), "borrowed"));
    let result = sma! {
        @macro_args(exclude = (borrowed));
        match 2 { 1 || 2 => borrowed!(@@"matched"), _ => "other" }
    };
    assert_eq!(result, "matched");
    assert_eq!(configured_module::owned_string(), "loaded outside Rust");
}

/// Parameters compose with attributes forwarded by an inner attribute.
#[test]
fn composes_with_attribute_forwarding() {
    assert_eq!(Packed::Pair((1, 2)), Packed::Pair((1, 2)));
    assert_eq!(AlsoPacked::Pair((1, 2)), AlsoPacked::Pair((1, 2)));
    strutuct! {
        @macro_args(prod);
        #[strutuct(no_prod)] Local Pair(u8, u8)
    }
    assert!(matches!(Local::Pair(1, 2), Local::Pair(1, 2)));
}

/// Expression and type positions accept the prefix directly.
#[test]
fn accepts_direct_parameter_prefixes() {
    assert_eq!(stringify_as!(@macro_args(snake); SomeName), "some_name");
    let text: qf!(@macro_args(); String) = perm!(@macro_args(); (), { String::from("fixed") });
    assert_eq!(text, "fixed");
    macro_rules! identity {
        ($value:expr) => {
            $value
        };
    }
    assert_eq!(reflect!(@macro_args(); identity, identity; 42), 42);
}
