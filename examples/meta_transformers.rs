//! Demonstrates transformations over macro invocations and token trees.

use these_macros_should_be_illegal::{perm, reflect};

/// Adds one after evaluating its input expression.
macro_rules! add_one {
    ($expression:expr) => {
        1 + $expression
    };
}

/// Doubles the value of its input expression.
macro_rules! double {
    ($expression:expr) => {
        2 * $expression
    };
}

fn main() {
    assert_eq!(add_one!(double!(3)), 7);
    assert_eq!(reflect!(add_one, double; 3), 8);

    // An empty cycle product is the identity. Nontrivial `perm!` invocations
    // deliberately expand to a raw comma-separated token stream; the book
    // shows that structural contract directly.
    let fixed: &str = perm!((), "fixed");
    assert_eq!(fixed.len(), 5);
}
