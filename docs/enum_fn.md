Generates an enum method as an inline match expression.

The attribute argument names the generated method and gives its return type.
An expression after a variant becomes that match arm's value:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::enum_fn;

#[enum_fn(description: &'static str)]
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

# Field projections

An integer RHS selects a zero-based tuple field and an identifier matching a
named field selects that field. The generated method borrows `self`, so the
selected expression is a shared field binding:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::enum_fn;

#[enum_fn(description: &str)]
enum Message<'a> {
    Fixed = "fixed text",
    Owned(String) = 0,
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

The field may have any type; Rust checks it against the declared return type.
Borrowed results are tied to the borrow of `self`.

A closure RHS instead receives every field in declaration order and computes
the result from the complete product. Any number of arguments,
including zero, is supported. Rust checks its arity, body, and return type:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::enum_fn;

#[enum_fn(description: String)]
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

# Missing arms

If any variant has no RHS, the declared return type is lifted into `Option`.
When every present expression is const-compatible, the optional method remains
a `const fn`:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::enum_fn;

#[enum_fn(description: &'static str)]
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
use these_macros_should_be_illegal::enum_fn;

#[enum_fn(description: &'static str = stringify)]
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

# Generic enums and ordinary Rust structure

The original enum declaration remains an ordinary `syn::ItemEnum`. Attributes,
visibility, generics, bounds, lifetimes, where clauses, and field types are
retained. Conditional attributes are copied to the corresponding generated
match arms so the enum and method stay exhaustive under the same configuration.

```rust
use these_macros_should_be_illegal::enum_fn;

#[enum_fn(size: usize)]
pub(crate) enum Value<'a, T: AsRef<str>, const N: usize>
where
    T: core::fmt::Debug,
{
    Text(&'a T) = |text| (*text).as_ref().len(),
    Bytes([u8; N]) = |bytes| bytes.len(),
    Function(fn(u8) -> u8) = |function| usize::from(function(2)),
    #[cfg(any())]
    Disabled = 0,
}

fn increment(value: u8) -> u8 {
    value + 1
}

let text = String::from("hello");
assert_eq!(Value::<String, 3>::Text(&text).size(), 5);
assert_eq!(Value::<String, 3>::Bytes([1, 2, 3]).size(), 3);
assert_eq!(Value::<String, 3>::Function(increment).size(), 3);
```

Tuple fields, arrays, raw pointers, function pointers, associated-type
projections, and blocks are ordinary Rust in this input. An integer selector is
validated against the selected tuple product before code generation.

# Arbitrary output types

The declared type is arbitrary. Expressions and closures must produce it
themselves; `enum_fn` performs no string conversion or cloning. Optionality is
inferred independently from the declared return type:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::enum_fn;

#[enum_fn(description: String)]
enum Error {
    Message(String) = |message| (*message).clone(),
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

An explicitly const closure uses `const { |arguments| expression }`. Its body
is inlined into the match arm and therefore does not downgrade the generated
method from `const fn`; an ordinary closure does.
