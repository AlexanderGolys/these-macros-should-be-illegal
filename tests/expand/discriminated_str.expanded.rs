#![no_main]
use these_macros_should_be_illegal::discriminated_str;
pub enum Token<'a, T> {
    Ident(&'a str),
    Owned(T),
    End,
}
impl<'a, T> Token<'a, T> {
    ///Returns this value's unique `name` discriminant.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Ident(..) => "ident",
            Self::Owned(..) => "owned",
            Self::End => "end",
        }
    }
}
