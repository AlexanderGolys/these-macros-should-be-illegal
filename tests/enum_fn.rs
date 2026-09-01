//! Consumer tests for enum methods generated from per-variant expressions.

extern crate alloc;

use alloc::string;
use these_macros_should_be_illegal::enum_fn;

/// A projected field type used to exercise qualified associated types.
trait HasItem {
    /// Value carried by the projection.
    type Item;
}

/// Concrete owner of the projected test type.
struct Carrier;

impl HasItem for Carrier {
    type Item = u16;
}

/// Constant expression referenced by a variant arm.
const CONST_SCORE: usize = 7;

/// Static expression referenced by a variant arm.
static STATIC_SCORE: usize = 11;

mod some_other_path {
    //! A deliberately distinct type whose final identifier is `String`.

    /// A non-text type that must not be inferred as the standard string.
    pub struct String;
}

#[enum_fn(description: &'static str)]
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

#[enum_fn(label: string::String)]
enum OwnedLabel {
    Value = String::from("owned"),
    Dynamic(string::String) = |value| (*value).clone(),
}

#[enum_fn(label: &str)]
enum BorrowedLabel {
    Value = "borrowed",
    Dynamic(String) = 0,
}

#[enum_fn(description: &str)]
enum MixedDescription {
    Tuple(u16) = "tuple payload",
    Struct { code: u8 } = "struct payload",
    Dynamic(String) = 0,
}

#[enum_fn(description: &str)]
enum SelectedDescription<'a> {
    Tuple(String, String) = 1,
    NestedTuple((String, String)) = 1,
    Struct { a: String, b: String } = a,
    Borrowed(&'a str) = 0,
    BorrowedTuple(u8, &'a str) = 1,
    BorrowedStruct { code: u8, text: &'a str } = text,
}

#[enum_fn(description: &str)]
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

/// Explicitly const closures keep the generated method const-callable.
#[enum_fn(description: &str)]
enum ConstClosureDescription<'a> {
    Selected(&'a str, &'a str) = const { |_, selected| *selected },
    Unit = const { || "unit" },
}

#[enum_fn(description: String)]
enum OwnedClosureDescription {
    Joined(String, String) = |left, right| format!("{left}:{right}"),
}

/// A deliberately non-string result proving the generated method is generic.
#[enum_fn(weight: usize)]
enum Weighted {
    Fixed = 3,
    Sum(usize, usize) = |left, right| *left + *right,
}

#[enum_fn(description: &str)]
enum LifetimeDescription<'a, 'b, 'c> {
    A(&'a str) = 0,
    B(&'b str) = 0,
    C(&'c str) = "a",
}

#[enum_fn(description: &str)]
enum OptionalDescription<'a> {
    Fixed = "fixed",
    Dynamic(String) = 0,
    Borrowed(&'a str) = 0,
    Closure(&'a str, &'a str) = |_, text| *text,
    Missing,
    Payload(u8),
}

/// Descriptions that are either compile-time constants or absent.
#[enum_fn(description: &'static str)]
enum OptionalFixedDescription {
    Fixed = "fixed",
    Missing,
}

/// An explicitly borrowed fixed accessor remains usable in constants.
#[enum_fn(description: &str)]
enum ExplicitBorrowedFixedDescription {
    Fixed = "fixed",
}

#[enum_fn(description: &str)]
enum DeliberatelyDistinctString {
    Value(some_other_path::String),
}

#[enum_fn(description: String)]
enum OptionalOwnedDescription {
    Fixed = String::from("owned fixed"),
    Dynamic(String) = |value| (*value).clone(),
    Missing,
}

#[enum_fn(description: &'static str = stringify)]
enum StringifiedDescription {
    Fixed = "fixed override",
    Missing,
    Payload(u8),
}

#[enum_fn(description: &'static str = panic)]
enum PanickingDescription {
    Missing,
}

/// Generic enum shape covering ordinary Rust field types and where clauses.
#[allow(dead_code)]
#[enum_fn(score: usize)]
pub(crate) enum GenericShapes<'a, T: AsRef<str>, const N: usize>
where
    T: core::fmt::Debug,
{
    Borrowed(&'a T) = |value| (*value).as_ref().len(),
    Pointer(*const T) = |pointer| usize::from(pointer.is_null()),
    Array([u8; N]) = |values| values.len(),
    Tuple((u8, u8)) = |pair| usize::from(pair.0) + usize::from(pair.1),
    Function(fn(u8) -> u8) = |function| usize::from(function(2)),
    Projected(<Carrier as HasItem>::Item) = |value| usize::from(*value),
}

/// Conditional attributes are forwarded to their generated match arms.
#[enum_fn(score: usize)]
enum ConditionalVariant {
    #[cfg(any())]
    Removed = 0,
    Present = 1,
}

/// Const, static, and block expressions remain ordinary Rust expressions.
#[enum_fn(score: usize)]
enum ExpressionShapes {
    Constant = CONST_SCORE,
    Static = STATIC_SCORE,
    Block = {
        let base = 6;
        base * 2
    },
}

const STRINGIFIED_MISSING: &str = StringifiedDescription::Missing.description();

const QUIT_DESCRIPTION: &str = Action::Quit.description();

/// A const closure evaluated through the generated const method.
const CONST_CLOSURE_DESCRIPTION: &str =
    ConstClosureDescription::Selected("ignored", "selected").description();

/// A present optional description evaluated in a constant context.
const OPTIONAL_FIXED_DESCRIPTION: Option<&str> = OptionalFixedDescription::Fixed.description();

/// An absent optional description evaluated in a constant context.
const OPTIONAL_MISSING_DESCRIPTION: Option<&str> = OptionalFixedDescription::Missing.description();

/// An explicit `&str` does not disable `const` when every value is constant.
const EXPLICIT_BORROWED_FIXED_DESCRIPTION: &str =
    ExplicitBorrowedFixedDescription::Fixed.description();

#[test]
fn generates_enum_method() {
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
    assert_eq!(CONST_CLOSURE_DESCRIPTION, "selected");
    assert_eq!(ConstClosureDescription::Unit.description(), "unit");
    assert_eq!(
        OwnedClosureDescription::Joined("left".into(), "right".into()).description(),
        "left:right",
    );
    assert_eq!(Weighted::Fixed.weight(), 3);
    assert_eq!(Weighted::Sum(4, 5).weight(), 9);
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
#[should_panic(expected = "variant `Missing` has no generated value")]
fn supports_panicking_missing_descriptions() {
    PanickingDescription::Missing.description();
}

/// Retains generics, bounds, lifetimes, const parameters, and rich field types.
#[test]
fn supports_complete_generic_enum_shapes() {
    fn increment(value: u8) -> u8 {
        value + 1
    }

    let text = String::from("borrowed");
    assert_eq!(GenericShapes::<String, 3>::Borrowed(&text).score(), 8);
    assert_eq!(
        GenericShapes::<String, 3>::Pointer(core::ptr::null()).score(),
        1
    );
    assert_eq!(GenericShapes::<String, 3>::Array([1, 2, 3]).score(), 3);
    assert_eq!(GenericShapes::<String, 3>::Tuple((4, 5)).score(), 9);
    assert_eq!(GenericShapes::<String, 3>::Function(increment).score(), 3);
    assert_eq!(GenericShapes::<String, 3>::Projected(13).score(), 13);
}

/// Keeps conditional arms synchronized and evaluates all expression categories.
#[test]
fn supports_conditional_and_ordinary_rust_expressions() {
    assert_eq!(ConditionalVariant::Present.score(), 1);
    assert_eq!(ExpressionShapes::Constant.score(), 7);
    assert_eq!(ExpressionShapes::Static.score(), 11);
    assert_eq!(ExpressionShapes::Block.score(), 12);
}

/// The enum and generated inherent implementation may both be block-local.
#[test]
fn expands_on_block_local_items() {
    #[enum_fn(value: usize)]
    enum LocalValue {
        Present = 3,
    }

    assert_eq!(LocalValue::Present.value(), 3);
}
