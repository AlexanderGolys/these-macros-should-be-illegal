#![no_main]

use these_macros_should_be_illegal::enum_fn;

#[enum_fn(pub const description -> &'static str)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit = "quit spectral-m2",
    Submit = "evaluate the input",
    InsertNewline = "insert a line break",
}

#[enum_fn(description -> &str)]
pub enum Message {
    Tuple(u16) = "tuple payload",
    Struct { code: u8 } = "struct payload",
    Dynamic(String) = |value| value.as_str(),
}

#[enum_fn(description -> &str)]
pub enum Selected<'a> {
    Tuple(String, String) = |_, value| value.as_str(),
    Struct { a: String, b: String } = |a, b| { let _ = b; a.as_str() },
    Borrowed(&'a str) = |value| *value,
}

#[enum_fn(description -> &str)]
pub enum Computed<'a> {
    Tuple(&'a str, &'a str) = |_, text| *text,
    Named { code: u8, text: &'a str } = |code, text| { let _ = code; *text },
    Empty {} = || "empty",
    Unit = || "unit",
}

#[enum_fn(description -> Option<&str>, missing = None)]
pub enum Optional<'a> {
    Fixed = Some("fixed"),
    Borrowed(&'a str) = |value| Some(*value),
    Missing,
    Payload(u8),
}

#[enum_fn(description -> Option<&'static str>, missing = None)]
pub enum OptionalFixed {
    Fixed = Some("fixed"),
    Missing,
}

#[enum_fn(pub(crate) description -> &'static str, missing = "fallback")]
pub enum FixedFallback {
    Fixed = "fixed override",
    Missing,
    Payload(u8),
}

#[non_exhaustive]
#[enum_fn(code -> u8, missing = 255)]
pub enum OuterNonExhaustive {
    Known = 7,
    Missing,
}

#[enum_fn(code -> u8, missing = 255)]
#[non_exhaustive]
pub enum InnerNonExhaustive {
    Known = 9,
    Missing,
}

#[enum_fn(code -> u8, missing = panic!("missing code"))]
pub enum Panicking {
    Known = 3,
}
