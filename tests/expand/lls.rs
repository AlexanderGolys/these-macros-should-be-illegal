#![no_main]

use these_macros_should_be_illegal::{excluded_macros, literally_literal_string};

#[excluded_macros(macro_excluded)]
literally_literal_string!(
    macro_rules! macro_excluded {
        (@@"a") => { "a was a string" };
        (@@ "b") => { "b was also a string" };
        (@@$s:literal) => { @@"not literally literal, but a string" };
        ($s:literal) => { "just a slice" };
        ($s:expr) => { @@ "a string all along" };
    }

    macro_rules! macro_included {
        (@@"a") => { "a was a string" };
        (@@ "b") => { "b was also a string" };
        (@@$s:literal) => { Infallible::<()> };
        ($s:literal) => { @@"Once there was some slice here" };
        ($s:expr) => { "back to a slice, again" };
    }

    pub fn owned_string() -> String {
        macro_excluded!(@@ "a");
        macro_excluded!(@@"b");
        macro_excluded!(@@"c");
        macro_excluded!("b");
        macro_excluded!(::std::string::String::from("a"));


        macro_included!(@@ "a");
        macro_included!(@@"b");
        macro_included!(@@"c");
        macro_included!("b");
        macro_included!(::std::string::String::from("a"));

        "__just a slice";
        @@"this is a string tho";
        @@ "and this is also a string"
    }
);
