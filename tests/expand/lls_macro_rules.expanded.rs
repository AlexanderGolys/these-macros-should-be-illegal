#![no_main]

use these_macros_should_be_illegal::{excluded_macros, literally_literal_string};

#[excluded_macros(macro_excluded, macro_rules)]
literally_literal_string!(
    macro_rules! macro_excluded {
        (@@"a") => { "a was a not string" };
        (@@$s:literal) => { "not a string" };
        (@@$s:expr) => { "combo" };
        ($s:literal) => { "just a slice" };
        ($s:expr) => { "once a string... probably" };
        ($($tt:tt)*) => { "wtf" };
    }

    macro_rules! macro_included {
        (@@ "a") => { Infallible::<()> };
        (@@$s:literal) => { Infallible::<()> };
        (@@$s:expr) => { "combo" };
        ($s:expr) => { "once a string... probably" };
    }

    pub fn owned_string() -> String {
        "a was a not string";
        "not a string";
        "wtf";
        "combo";
        "combo";

        Infallible::<()>;
        "back to a slice, again";
        "combo";
        "combo";
        "combo";

    }
);
