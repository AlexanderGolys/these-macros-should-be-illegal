Generates an accessor from string-like enum discriminants and fields.

The attribute argument names the generated method. A string literal after a
variant becomes its fixed value:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::discriminated_str;

#[discriminated_str(description)]
enum Action {
    Quit = "quit the application",
    Submit = "evaluate the input",
}

const QUIT: &str = Action::Quit.description();
assert_eq!(QUIT, "quit the application");
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
enum Action {
    Quit,
    Submit,
}

impl Action {
    const fn description(&self) -> &'static str {
        match self {
            Self::Quit => "quit the application",
            Self::Submit => "evaluate the input",
        }
    }
}

const QUIT: &str = Action::Quit.description();
assert_eq!(QUIT, "quit the application");
```

</div>

</div>

# Dynamic descriptions and projections

A sole `String` or `&str` field supplies a dynamic value automatically. For a
variant with multiple fields, an integer discriminant selects a zero-based
tuple field and an identifier selects a named field:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::discriminated_str;

#[discriminated_str(description)]
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

</div>

<div class="highlight-comparison-pane">

```rust,ignore
enum Message<'a> {
    Fixed,
    Owned(String),
    Pair(u8, &'a str),
    Named { code: u8, text: String },
}

impl Message<'_> {
    fn description(&self) -> &str {
        match self {
            Self::Fixed => "fixed text",
            Self::Owned(value) => value,
            Self::Pair(_, value) => value,
            Self::Named { text, .. } => text,
        }
    }
}

assert_eq!(Message::Owned("owned".into()).description(), "owned");
assert_eq!(Message::Pair(7, "second").description(), "second");
```

</div>

</div>

Selected fields must have type `String` or `&str`. The borrowed result is tied
to the borrow of `self`, so an enum may use distinct field lifetimes.

A closure discriminant instead receives every field in declaration order and
computes the description from the complete product. Any number of arguments,
including zero, is supported. Rust checks its arity, body, and return type:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::discriminated_str;

#[discriminated_str(description: String)]
enum Message {
    Pair(String, String) = |left, right| format!("{left}: {right}"),
    Unit = || String::from("unit"),
}

assert_eq!(
    Message::Pair("hello".into(), "world".into()).description(),
    "hello: world",
);
assert_eq!(Message::Unit.description(), "unit");
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
enum Message {
    Pair(String, String),
    Unit,
}

impl Message {
    fn description(&self) -> String {
        match self {
            Self::Pair(left, right) => format!("{left}: {right}"),
            Self::Unit => String::from("unit"),
        }
    }
}

assert_eq!(
    Message::Pair("hello".into(), "world".into()).description(),
    "hello: world",
);
```

</div>

</div>

Because the generated accessor matches on `&self`, closure arguments receive
the corresponding shared field bindings.

# Missing descriptions

If any variant has neither a fixed description nor a selected or inferred text
field, the accessor returns `Option`. When every present description is fixed,
the optional accessor remains a `const fn` returning `Option<&'static str>`:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::discriminated_str;

#[discriminated_str(description)]
enum Status {
    Ready = "ready",
    Unknown,
}

const READY: Option<&str> = Status::Ready.description();
const UNKNOWN: Option<&str> = Status::Unknown.description();

assert_eq!(READY, Some("ready"));
assert_eq!(UNKNOWN, None);
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
enum Status {
    Ready,
    Unknown,
}

impl Status {
    const fn description(&self) -> Option<&'static str> {
        match self {
            Self::Ready => Some("ready"),
            Self::Unknown => None,
        }
    }
}

const READY: Option<&str> = Status::Ready.description();
const UNKNOWN: Option<&str> = Status::Unknown.description();
```

</div>

</div>

Use `= stringify` on the attribute to return a missing variant's Rust name
instead:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::discriminated_str;

#[discriminated_str(description = stringify)]
enum Status {
    Ready = "ready",
    Unknown,
}

assert_eq!(Status::Unknown.description(), "Unknown");
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
enum Status {
    Ready,
    Unknown,
}

impl Status {
    const fn description(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Unknown => "Unknown",
        }
    }
}

assert_eq!(Status::Unknown.description(), "Unknown");
```

</div>

</div>

# Owned output

Write `: String` after the method name to clone or allocate each present value.
Optionality remains inferred independently from that base return type:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::discriminated_str;

#[discriminated_str(description: String)]
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

</div>

<div class="highlight-comparison-pane">

```rust,ignore
enum Error {
    Message(String),
    Missing,
}

impl Error {
    fn description(&self) -> Option<String> {
        match self {
            Self::Message(value) => Some(value.clone()),
            Self::Missing => None,
        }
    }
}

assert_eq!(Error::Missing.description(), None);
```

</div>

</div>

An explicit `: &str` is also accepted, although it is equivalent to the
default whenever a dynamic description is present.
