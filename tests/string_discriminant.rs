//! Consumer tests for fixed and dynamic string-discriminant accessors.

use these_macros_should_be_illegal::str_disc;

#[str_disc(description)]
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

#[str_disc(label: String)]
enum OwnedLabel {
    Value = "owned",
    Dynamic(String),
}

#[str_disc(label: &str)]
enum BorrowedLabel {
    Value = "borrowed",
    Dynamic(String),
}

#[str_disc(description)]
enum MixedDescription {
    Tuple(u16) = "tuple payload",
    Struct { code: u8 } = "struct payload",
    Dynamic(String),
}

const QUIT_DESCRIPTION: &str = Action::Quit.description();

#[test]
fn generates_string_discriminant_accessor() {
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
