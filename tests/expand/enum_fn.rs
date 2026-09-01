#![no_main]

use these_macros_should_be_illegal::enum_fn;

#[enum_fn(description: &'static str)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit = "quit spectral-m2",
    Submit = "evaluate the input",
    InsertNewline = "insert a line break",
}

#[enum_fn(description: &str)]
pub enum Message {
    Tuple(u16) = "tuple payload",
    Struct { code: u8 } = "struct payload",
    Dynamic(String) = 0,
}

#[enum_fn(description: &str)]
pub enum Selected<'a> {
    Tuple(String, String) = 1,
    Struct { a: String, b: String } = a,
    Borrowed(&'a str) = 0,
}

#[enum_fn(description: &str)]
pub enum Computed<'a> {
    Tuple(&'a str, &'a str) = |_, text| *text,
    Named { code: u8, text: &'a str } = |_, text| *text,
    Unit = || "unit",
}

#[enum_fn(description: &str)]
pub enum ConstComputed<'a> {
    Tuple(&'a str, &'a str) = const { |_, text| *text },
    Unit = const { || "unit" },
}

#[enum_fn(description: &str)]
pub enum Optional<'a> {
    Fixed = "fixed",
    Borrowed(&'a str) = 0,
    Missing,
    Payload(u8),
}

#[enum_fn(description: &'static str)]
pub enum OptionalFixed {
    Fixed = "fixed",
    Missing,
}

#[enum_fn(description: &'static str = stringify)]
pub enum Stringified {
    Fixed = "fixed override",
    Missing,
    Payload(u8),
}
