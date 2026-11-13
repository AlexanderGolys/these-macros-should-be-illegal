#![no_main]
use these_macros_should_be_illegal::literally_literal_string;
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
