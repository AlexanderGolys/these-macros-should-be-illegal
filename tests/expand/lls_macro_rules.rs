#![no_main]

use these_macros_should_be_illegal::{excluded_macros, literally_literal_string};

#[excluded_macros(macro_excluded, macro_rules)]
literally_literal_string!(
    macro_rules! macro_excluded {
        (@@ "a") => { "a was a not string" };
        (@@$s:literal) => { "not a string" };
        (@@$s:expr) => { "combo" };
        ($s:literal) => { "just a slice" };
        ($s:expr) => { "once a string... probably" };
        ($($tt:tt)*) => { "wtf" };
    }

    macro_rules! macro_included {
        (@ @ $s:literal) => { Infallible::<()> };
        (@@ "a") => { Infallible::<()> };
        (@@$s:expr) => { "combo" };
        ($s:literal) => { "just a slice" };
        ($s:expr) => { "back to a slice, again" };
    }

    pub fn owned_string() -> String {
        macro_excluded!(@@"a");
        macro_excluded!(@ @"b");
        macro_excluded!(@@ @@"c");
        macro_excluded!(@@ std::string::String::from("a"));
        macro_excluded!(@@ ::std::string::String::from("a"));


        macro_included!(@  @ "a");
        macro_included!(@@"b");
        macro_included!(@@ @@"c");
        macro_included!(@@ std::string::String::from("a"));
        macro_included!(@@ ::std::string::String::from("a"));

    }
);
