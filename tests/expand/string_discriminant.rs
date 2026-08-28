#![no_main]

use these_macros_should_be_illegal::str_disc;

#[str_disc(description)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit = "quit spectral-m2",
    Submit = "evaluate the input",
    InsertNewline = "insert a line break",
}

#[str_disc(description)]
pub enum Message {
    Tuple(u16) = "tuple payload",
    Struct { code: u8 } = "struct payload",
    Dynamic(String),
}

#[str_disc(description)]
pub enum Selected<'a> {
    Tuple(String, String) = 1,
    Struct { a: String, b: String } = a,
    Borrowed(&'a str),
}

#[str_disc(description)]
pub enum Optional<'a> {
    Fixed = "fixed",
    Borrowed(&'a str),
    Missing,
    Payload(u8),
}

#[str_disc(description)]
pub enum OptionalFixed {
    Fixed = "fixed",
    Missing,
}

#[str_disc(description = stringify)]
pub enum Stringified {
    Fixed = "fixed override",
    Missing,
    Payload(u8),
}
