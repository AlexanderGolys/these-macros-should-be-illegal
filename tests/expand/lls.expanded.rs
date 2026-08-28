#![no_main]
use these_macros_should_be_illegal::{excluded_macros, literally_literal_string};
pub fn owned_string() -> String {
    ::std::string::String::from("not literally literal, but a string" );
    ::std::string::String::from("not literally literal, but a string" );
    ::std::string::String::from("not literally literal, but a string" );
    "just a slice";
    "a was a string";

    "a was a string";
    "b was also a string";
    "back to a slice, again";
    ::std::string::String::from("Once there was some slice here" );
    "a was a string";

    "__just a slice";
    ::std::string::String::from("this is a string tho");
    ::std::string::String::from("and this is also a string")
}
