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
