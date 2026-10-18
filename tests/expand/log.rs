#![no_main]

use these_macros_should_be_illegal::{Log, forall};

pub struct Style(u8);

#[derive(Log)]
#[allow(non_snake_case)]
pub struct BaseGroups {
    /// Style of a keyword.
    pub Keyword: Style,
    pub Bg: Style,
}

// The `log` helper attribute stays on the struct, so an expanded fixture cannot
// carry it: the declared name is covered by the unit and consumer tests instead.
#[forall(pub, Style)]
#[derive(Log)]
#[allow(non_snake_case)]
pub struct Theme(Keyword, Function);

#[derive(Log)]
struct Mixed {
    count: u8,
    name: &'static str,
}

#[derive(Log)]
struct Nothing;
