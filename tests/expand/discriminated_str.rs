#![no_main]

use these_macros_should_be_illegal::discriminated_str;

#[discriminated_str(description)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit = "quit spectral-m2",
    Submit = "evaluate the input",
    InsertNewline = "insert a line break",
}

#[discriminated_str(description)]
pub enum Message {
    Tuple(u16) = "tuple payload",
    Struct { code: u8 } = "struct payload",
    Dynamic(String),
}

#[discriminated_str(description)]
pub enum Selected<'a> {
    Tuple(String, String) = 1,
    Struct { a: String, b: String } = a,
    Borrowed(&'a str),
}

#[discriminated_str(description)]
pub enum Computed<'a> {
    Tuple(&'a str, &'a str) = |_, text| *text,
    Named { code: u8, text: &'a str } = |_, text| *text,
    Unit = || "unit",
}

#[discriminated_str(description)]
pub enum Optional<'a> {
    Fixed = "fixed",
    Borrowed(&'a str),
    Missing,
    Payload(u8),
}

#[discriminated_str(description)]
pub enum OptionalFixed {
    Fixed = "fixed",
    Missing,
}

#[discriminated_str(description = stringify)]
pub enum Stringified {
    Fixed = "fixed override",
    Missing,
    Payload(u8),
}
