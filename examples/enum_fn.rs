//! Generates one method from expressions attached directly to enum variants.

use these_macros_should_be_illegal::enum_fn;

/// Command labels generated from constant, computed, and missing arms.
#[enum_fn(label: String)]
enum Command<'a> {
    /// A fixed owned label.
    Quit = String::from("quit"),
    /// A label computed from the borrowed payload.
    Named(&'a str) = |name| format!("run {name}"),
    /// A deliberately absent label.
    Hidden,
}

fn main() {
    assert_eq!(Command::Quit.label(), Some(String::from("quit")));
    assert_eq!(
        Command::Named("checks").label(),
        Some(String::from("run checks")),
    );
    assert_eq!(Command::Hidden.label(), None);
}
