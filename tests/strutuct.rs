//! Consumer tests for nested algebraic declarations and constructor macros.

use these_macros_should_be_illegal::strutuct;

/// Leaf payload used by the nested-struct consumer test.
pub struct B(pub u8);

strutuct! {
    S
    a: A { A1, A2, A3 },
    b: B,
}

/// Prefix-expression payload used to exercise transitive construction.
pub struct Pref(pub u8);
/// Postfix-expression payload used to exercise transitive construction.
pub struct Post(pub u8);
/// Binary-expression payload used to exercise direct construction.
pub struct Bin(pub u8);

strutuct! {
    Expr
    Unary { (Pref), (Post) }
    (Bin)
    LitStr(String)
    Null
}

strutuct! {
    Message
    Payload { value: String }
    Empty
}

strutuct! {
    Wrapped
    optional: Pref?,
    boxed: Post*,
}

strutuct! {
    Outer
    Middle { Inner { Leaf { Done(String) } } }
}

/// Verifies that nested declarations are emitted before their parent struct.
#[test]
fn hoists_nested_declarations_into_ordinary_public_types() {
    let value = S!(a: A!(A2), b: B(7));

    assert!(matches!(value.a, A::A2));
    assert_eq!(value.b.0, 7);
}

/// Verifies direct construction from leaves through multiple generated enums.
#[test]
fn composes_automatic_constructors_through_nested_enums() {
    let prefix = Expr!(Unary::Pref(1));
    let postfix = Expr!(Unary::Post(2));
    let binary = Expr!(Bin(3));

    assert!(matches!(prefix, Expr::ExprUnary(Unary::UnaryPref(Pref(1)))));
    assert!(matches!(
        postfix,
        Expr::ExprUnary(Unary::UnaryPost(Post(2)))
    ));
    assert!(matches!(binary, Expr::ExprBin(Bin(3))));
}

/// Verifies that explicitly named Rust-like variants retain their constructors.
#[test]
fn leaves_explicit_enum_constructors_ordinary() {
    let literal = Expr!(LitStr("text".to_owned()));
    let null = Expr!(Null);

    assert!(matches!(literal, Expr::LitStr(value) if value == "text"));
    assert!(matches!(null, Expr::Null));
}

/// Verifies nested structs as variants and both postfix type transformations.
#[test]
fn constructs_nested_struct_variants_and_postfix_types() {
    let message = Message!(Payload {
        value: "payload".to_owned()
    });
    let wrapped = Wrapped!(
        optional: Some(Pref(4)),
        boxed: Box::new(Post(5)),
    );

    assert!(matches!(
        message,
        Message::MessagePayload(Payload { value }) if value == "payload"
    ));
    assert!(matches!(wrapped.optional, Some(Pref(4))));
    assert_eq!(wrapped.boxed.0, 5);
}

/// Verifies that every generated enum macro consumes exactly one path segment.
#[test]
fn folds_a_complete_nested_enum_path() {
    let value = Outer!(Middle::Inner::Leaf::Done("finished".to_owned()));

    assert!(matches!(
        value,
        Outer::OuterMiddle(Middle::MiddleInner(Inner::InnerLeaf(Leaf::Done(message))))
            if message == "finished"
    ));
}
