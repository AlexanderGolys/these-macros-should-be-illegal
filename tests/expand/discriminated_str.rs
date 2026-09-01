#![no_main]

use these_macros_should_be_illegal::discriminated_str;

#[discriminated_str(name)]
pub enum Token<'a, T> {
    Ident(&'a str) = "ident",
    Owned(T) = "owned",
    End = "end",
}
