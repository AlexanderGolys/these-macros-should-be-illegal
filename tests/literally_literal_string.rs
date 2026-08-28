//! Consumer tests for literal-string rewriting and preprocessing exclusions.

use these_macros_should_be_illegal::{excluded_macros, literally_literal_string};

macro_rules! raw_tokens {
    ($($tokens:tt)*) => {
        "borrowed"
    };
}

#[excluded_macros(raw_tokens)]
literally_literal_string! {
    fn configured_macro_arguments_are_opaque() -> &'static str {
        raw_tokens!(@@"untouched")
    }
}

these_macros_should_be_illegal::expand!(
    these_macros_should_be_illegal::literally_literal_string,
    exclude_macros = (raw_tokens);
    #[path = "fixtures/literally_literal_string_module.rs"]
    mod literally_literal_string_module;
);

#[test]
fn function_like_macro_rewrites_only_its_input() {
    let value = literally_literal_string!(@@"direct invocation");

    assert_eq!(value, "direct invocation");
}

#[test]
fn shared_expander_loads_and_injects_the_selected_macro() {
    assert_eq!(
        literally_literal_string_module::owned_string(),
        "loaded outside Rust"
    );
}

#[test]
fn shared_config_excludes_macro_inputs() {
    assert_eq!(configured_macro_arguments_are_opaque(), "borrowed");
    assert_eq!(
        literally_literal_string_module::excluded_macro_arguments_are_opaque(),
        "borrowed"
    );
}

#[test]
fn ordinary_rust_is_unchanged() {
    assert_eq!(
        literally_literal_string_module::ordinary_rust_passes_through(),
        "still borrowed"
    );
}
