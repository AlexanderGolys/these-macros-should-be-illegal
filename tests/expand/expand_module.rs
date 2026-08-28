#![no_main]

these_macros_should_be_illegal::expand!(
    these_macros_should_be_illegal::literally_literal_string;
    #[path = "fixtures/expand_module_input.rs"]
    mod expanded;
);
