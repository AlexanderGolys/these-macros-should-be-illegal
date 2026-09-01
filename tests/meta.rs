//! Consumer tests for transformations of macro invocation structure.

use these_macros_should_be_illegal::reflect;

macro_rules! add_one {
    ($expression:expr) => {
        1 + $expression
    };
}

macro_rules! double {
    ($expression:expr) => {
        2 * $expression
    };
}

/// Reflection changes which invocation is expanded on the outside.
#[test]
fn reflects_macro_composition_order() {
    let ordinary = add_one!(double!(3));
    let reflected = reflect!(add_one, double; 3);

    assert_eq!(ordinary, 7);
    assert_eq!(reflected, 8);
}
