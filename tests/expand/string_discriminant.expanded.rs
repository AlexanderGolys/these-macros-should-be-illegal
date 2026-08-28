#![no_main]

use these_macros_should_be_illegal::str_disc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Submit,
    InsertNewline,
}

impl Action {
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Quit => "quit spectral-m2",
            Self::Submit => "evaluate the input",
            Self::InsertNewline => "insert a line break",
        }
    }
}

pub enum Message {
    Tuple(u16),
    Struct { code: u8 },
    Dynamic(String),
}

impl Message {
    pub fn description(&self) -> &str {
        match self {
            Self::Tuple(..) => "tuple payload",
            Self::Struct { .. } => "struct payload",
            Self::Dynamic(value) => value.as_str(),
        }
    }
}
