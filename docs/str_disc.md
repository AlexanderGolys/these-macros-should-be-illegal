Generates an accessor from string-like enum discriminants and fields.

The attribute argument names the generated method. A string literal after a
variant becomes its fixed value:

```rust
use these_macros_should_be_illegal::str_disc;

#[str_disc(description)]
enum Action {
    Quit = "quit the application",
    Submit = "evaluate the input",
}

const QUIT: &str = Action::Quit.description();
assert_eq!(QUIT, "quit the application");
```

# Dynamic descriptions and projections

A sole `String` or `&str` field supplies a dynamic value automatically. For a
variant with multiple fields, an integer discriminant selects a zero-based
tuple field and an identifier selects a named field:

```rust
use these_macros_should_be_illegal::str_disc;

#[str_disc(description)]
enum Message<'a> {
    Fixed = "fixed text",
    Owned(String),
    Pair(u8, &'a str) = 1,
    Named { code: u8, text: String } = text,
}

assert_eq!(Message::Owned("owned".into()).description(), "owned");
assert_eq!(Message::Pair(7, "second").description(), "second");
assert_eq!(
    Message::Named { code: 9, text: "named".into() }.description(),
    "named",
);
```

Selected fields must have type `String` or `&str`. The borrowed result is tied
to the borrow of `self`, so an enum may use distinct field lifetimes.

# Missing descriptions

If any variant has neither a fixed description nor a selected or inferred text
field, the accessor returns `Option`. When every present description is fixed,
the optional accessor remains a `const fn` returning `Option<&'static str>`:

```rust
use these_macros_should_be_illegal::str_disc;

#[str_disc(description)]
enum Status {
    Ready = "ready",
    Unknown,
}

const READY: Option<&str> = Status::Ready.description();
const UNKNOWN: Option<&str> = Status::Unknown.description();

assert_eq!(READY, Some("ready"));
assert_eq!(UNKNOWN, None);
```

Use `= stringify` on the attribute to return a missing variant's Rust name
instead:

```rust
use these_macros_should_be_illegal::str_disc;

#[str_disc(description = stringify)]
enum Status {
    Ready = "ready",
    Unknown,
}

assert_eq!(Status::Unknown.description(), "Unknown");
```

# Owned output

Write `: String` after the method name to clone or allocate each present value.
Optionality remains inferred independently from that base return type:

```rust
use these_macros_should_be_illegal::str_disc;

#[str_disc(description: String)]
enum Error {
    Message(String),
    Missing,
}

assert_eq!(
    Error::Message("details".into()).description(),
    Some(String::from("details")),
);
assert_eq!(Error::Missing.description(), None);
```

An explicit `: &str` is also accepted, although it is equivalent to the
default whenever a dynamic description is present.
