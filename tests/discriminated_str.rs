//! Consumer tests for fixed and dynamic string-discriminant accessors.

extern crate alloc;

use alloc::string;
use these_macros_should_be_illegal::discriminated_str;

mod some_other_path {
    //! A deliberately distinct type whose final identifier is `String`.

    /// A non-text type that must not be inferred as the standard string.
    pub struct String;
}

#[discriminated_str(description)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Quit = "quit spectral-m2",
    Submit = "evaluate the input",
    InsertNewline = "insert a line break",
    ScrollFeedUp = "scroll the feed up",
    ScrollFeedDown = "scroll the feed down",
    OpenSettings = "open settings",
    OpenKeymap = "open key bindings",
    OpenThemes = "open colour schemes",
    MoveLeft = "move left",
    MoveRight = "move right",
    MoveUp = "move up, or recall older input",
    MoveDown = "move down, or recall newer input",
    MoveLineStart = "move to line start",
    MoveLineEnd = "move to line end",
    SelectLeft = "extend selection left",
    SelectRight = "extend selection right",
    SelectLineStart = "extend selection to line start",
    SelectLineEnd = "extend selection to line end",
    CopySelection = "copy the selection",
    DeleteBack = "delete before the cursor",
    DeleteForward = "delete under the cursor",
    InsertTab = "insert a tab",
}

#[discriminated_str(label: string::String)]
enum OwnedLabel {
    Value = "owned",
    Dynamic(string::String),
}

#[discriminated_str(label: &str)]
enum BorrowedLabel {
    Value = "borrowed",
    Dynamic(String),
}

#[discriminated_str(description)]
enum MixedDescription {
    Tuple(u16) = "tuple payload",
    Struct { code: u8 } = "struct payload",
    Dynamic(String),
}

#[discriminated_str(description)]
enum SelectedDescription<'a> {
    Tuple(String, String) = 1,
    NestedTuple((String, String)) = 1,
    Struct { a: String, b: String } = a,
    Borrowed(&'a str),
    BorrowedTuple(u8, &'a str) = 1,
    BorrowedStruct { code: u8, text: &'a str } = text,
}

#[discriminated_str(description)]
enum ClosureDescription<'a> {
    Longer(&'a str, &'a str) = |left, right| {
        if left.len() >= right.len() {
            *left
        } else {
            *right
        }
    },
    Named {
        code: u8,
        text: &'a str,
    } = |_, text| *text,
    Unit = || "unit",
}

#[discriminated_str(description: String)]
enum OwnedClosureDescription {
    Joined(String, String) = |left, right| format!("{left}:{right}"),
}

#[discriminated_str(description)]
enum LifetimeDescription<'a, 'b, 'c> {
    A(&'a str),
    B(&'b str),
    C(&'c str) = "a",
}

#[discriminated_str(description)]
enum OptionalDescription<'a> {
    Fixed = "fixed",
    Dynamic(String),
    Borrowed(&'a str),
    Closure(&'a str, &'a str) = |_, text| *text,
    Missing,
    Payload(u8),
}

/// Descriptions that are either compile-time constants or absent.
#[discriminated_str(description)]
enum OptionalFixedDescription {
    Fixed = "fixed",
    Missing,
}

/// An explicitly borrowed fixed accessor remains usable in constants.
#[discriminated_str(description: &str)]
enum ExplicitBorrowedFixedDescription {
    Fixed = "fixed",
}

#[discriminated_str(description)]
enum DeliberatelyDistinctString {
    Value(some_other_path::String),
}

#[discriminated_str(description: String)]
enum OptionalOwnedDescription {
    Fixed = "owned fixed",
    Dynamic(String),
    Missing,
}

#[discriminated_str(description = stringify)]
enum StringifiedDescription {
    Fixed = "fixed override",
    Missing,
    Payload(u8),
}

#[discriminated_str(description = panic)]
enum PanickingDescription {
    Missing,
}

const STRINGIFIED_MISSING: &str = StringifiedDescription::Missing.description();

const QUIT_DESCRIPTION: &str = Action::Quit.description();

/// A present optional description evaluated in a constant context.
const OPTIONAL_FIXED_DESCRIPTION: Option<&str> = OptionalFixedDescription::Fixed.description();

/// An absent optional description evaluated in a constant context.
const OPTIONAL_MISSING_DESCRIPTION: Option<&str> = OptionalFixedDescription::Missing.description();

/// An explicit `&str` does not disable `const` when every value is constant.
const EXPLICIT_BORROWED_FIXED_DESCRIPTION: &str =
    ExplicitBorrowedFixedDescription::Fixed.description();

#[test]
fn generates_discriminated_string_accessor() {
    let cases = [
        (Action::Quit, "quit spectral-m2"),
        (Action::Submit, "evaluate the input"),
        (Action::InsertNewline, "insert a line break"),
        (Action::ScrollFeedUp, "scroll the feed up"),
        (Action::ScrollFeedDown, "scroll the feed down"),
        (Action::OpenSettings, "open settings"),
        (Action::OpenKeymap, "open key bindings"),
        (Action::OpenThemes, "open colour schemes"),
        (Action::MoveLeft, "move left"),
        (Action::MoveRight, "move right"),
        (Action::MoveUp, "move up, or recall older input"),
        (Action::MoveDown, "move down, or recall newer input"),
        (Action::MoveLineStart, "move to line start"),
        (Action::MoveLineEnd, "move to line end"),
        (Action::SelectLeft, "extend selection left"),
        (Action::SelectRight, "extend selection right"),
        (Action::SelectLineStart, "extend selection to line start"),
        (Action::SelectLineEnd, "extend selection to line end"),
        (Action::CopySelection, "copy the selection"),
        (Action::DeleteBack, "delete before the cursor"),
        (Action::DeleteForward, "delete under the cursor"),
        (Action::InsertTab, "insert a tab"),
    ];

    for (action, expected) in cases {
        assert_eq!(action.description(), expected);
    }

    assert_eq!(QUIT_DESCRIPTION, "quit spectral-m2");
}

#[test]
fn supports_owned_and_borrowed_return_types_without_consuming_the_enum() {
    let owned = OwnedLabel::Value;
    let first: String = owned.label();
    let second: String = owned.label();
    assert_eq!(first, "owned");
    assert_eq!(second, "owned");

    let dynamic_owned = OwnedLabel::Dynamic("dynamic owned".to_owned());
    assert_eq!(dynamic_owned.label(), "dynamic owned");
    assert_eq!(dynamic_owned.label(), "dynamic owned");

    let borrowed = BorrowedLabel::Value;
    let first: &str = borrowed.label();
    let second: &str = borrowed.label();
    assert_eq!(first, "borrowed");
    assert_eq!(second, "borrowed");

    let dynamic_borrowed = BorrowedLabel::Dynamic("dynamic borrowed".to_owned());
    assert_eq!(dynamic_borrowed.label(), "dynamic borrowed");
    assert_eq!(dynamic_borrowed.label(), "dynamic borrowed");
}

#[test]
fn preserves_payloads_and_borrows_dynamic_descriptions() {
    let tuple = MixedDescription::Tuple(42);
    assert_eq!(tuple.description(), "tuple payload");
    assert!(matches!(tuple, MixedDescription::Tuple(42)));

    let structure = MixedDescription::Struct { code: 7 };
    assert_eq!(structure.description(), "struct payload");
    assert!(matches!(structure, MixedDescription::Struct { code: 7 }));

    let dynamic = MixedDescription::Dynamic("dynamic description".to_owned());
    assert_eq!(dynamic.description(), "dynamic description");
    assert_eq!(dynamic.description(), "dynamic description");
}

/// Selects tuple and named fields while borrowing explicit-lifetime strings.
#[test]
fn selects_dynamic_fields_and_preserves_enum_lifetimes() {
    let tuple = SelectedDescription::Tuple("first".to_owned(), "second".to_owned());
    assert_eq!(tuple.description(), "second");
    assert!(matches!(
        &tuple,
        SelectedDescription::Tuple(first, _) if first == "first"
    ));

    let nested = SelectedDescription::NestedTuple(("first".to_owned(), "second".to_owned()));
    assert_eq!(nested.description(), "second");
    assert!(matches!(
        &nested,
        SelectedDescription::NestedTuple((first, _)) if first == "first"
    ));

    let structure = SelectedDescription::Struct {
        a: "selected".to_owned(),
        b: "ignored".to_owned(),
    };
    assert_eq!(structure.description(), "selected");
    assert!(matches!(
        &structure,
        SelectedDescription::Struct { b, .. } if b == "ignored"
    ));

    let source = String::from("borrowed");
    assert_eq!(
        SelectedDescription::Borrowed(&source).description(),
        "borrowed"
    );
    let borrowed_tuple = SelectedDescription::BorrowedTuple(7, &source);
    assert_eq!(borrowed_tuple.description(), "borrowed");
    assert!(matches!(
        borrowed_tuple,
        SelectedDescription::BorrowedTuple(7, _)
    ));
    let borrowed_struct = SelectedDescription::BorrowedStruct {
        code: 9,
        text: &source,
    };
    assert_eq!(borrowed_struct.description(), "borrowed");
    assert!(matches!(
        borrowed_struct,
        SelectedDescription::BorrowedStruct { code: 9, .. }
    ));
}

/// Invokes closures with every field and trusts Rust to check their result types.
#[test]
fn computes_descriptions_from_variant_products() {
    assert_eq!(
        ClosureDescription::Longer("longer", "short").description(),
        "longer"
    );
    assert_eq!(
        ClosureDescription::Named {
            code: 7,
            text: "named",
        }
        .description(),
        "named",
    );
    assert_eq!(ClosureDescription::Unit.description(), "unit");
    assert_eq!(
        OwnedClosureDescription::Joined("left".into(), "right".into()).description(),
        "left:right",
    );
}

/// Uses the `&self` borrow as the common lifetime for distinct variant references.
#[test]
fn reborrows_distinct_variant_lifetimes_for_the_accessor() {
    let first = String::from("first");
    let second = String::from("second");
    let ignored = String::from("ignored");
    let a = LifetimeDescription::A(&first);
    let b = LifetimeDescription::B(&second);
    let c = LifetimeDescription::C(&ignored);

    assert_eq!(a.description(), "first");
    assert_eq!(b.description(), "second");
    assert_eq!(c.description(), "a");
    assert!(matches!(c, LifetimeDescription::C(value) if value == "ignored"));
}

/// Returns `None` only for variants without fixed or dynamic descriptions.
#[test]
fn supports_optional_borrowed_and_owned_descriptions() {
    assert_eq!(EXPLICIT_BORROWED_FIXED_DESCRIPTION, "fixed");
    assert_eq!(OPTIONAL_FIXED_DESCRIPTION, Some("fixed"));
    assert_eq!(OPTIONAL_MISSING_DESCRIPTION, None);
    assert_eq!(OptionalDescription::Fixed.description(), Some("fixed"));
    assert_eq!(
        OptionalDescription::Dynamic("dynamic".to_owned()).description(),
        Some("dynamic")
    );
    assert_eq!(
        OptionalDescription::Borrowed("borrowed").description(),
        Some("borrowed")
    );
    assert_eq!(
        OptionalDescription::Closure("ignored", "computed").description(),
        Some("computed")
    );
    assert_eq!(OptionalDescription::Missing.description(), None);
    let payload = OptionalDescription::Payload(7);
    assert_eq!(payload.description(), None);
    assert!(matches!(payload, OptionalDescription::Payload(7)));

    assert_eq!(
        OptionalOwnedDescription::Fixed.description(),
        Some("owned fixed".to_owned())
    );
    let dynamic = OptionalOwnedDescription::Dynamic("owned dynamic".to_owned());
    assert_eq!(dynamic.description(), Some("owned dynamic".to_owned()));
    assert_eq!(dynamic.description(), Some("owned dynamic".to_owned()));
    assert_eq!(OptionalOwnedDescription::Missing.description(), None);

    assert_eq!(
        StringifiedDescription::Fixed.description(),
        "fixed override"
    );
    assert_eq!(STRINGIFIED_MISSING, "Missing");
    let payload = StringifiedDescription::Payload(8);
    assert_eq!(payload.description(), "Payload");
    assert!(matches!(payload, StringifiedDescription::Payload(8)));
}

/// Does not confuse an unrelated qualified `String` with the standard type.
#[test]
fn preserves_deliberately_distinct_string_paths() {
    let value = DeliberatelyDistinctString::Value(some_other_path::String);
    assert_eq!(value.description(), None);
    assert!(matches!(value, DeliberatelyDistinctString::Value(_)));
}

/// Applies the explicitly requested panic fallback to undescribed variants.
#[test]
#[should_panic(expected = "variant `Missing` has no description")]
fn supports_panicking_missing_descriptions() {
    PanickingDescription::Missing.description();
}
