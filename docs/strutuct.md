# `strutuct!`

`strutuct!` keeps small related types where they are used, then hoists them into
ordinary Rust declarations in dependency order. Generated declarations and
fields are public by default. `emmun!` is an exact alias with the same syntax.

Outer attributes can configure the complete generated family through
[`forward_attributes`](forward-attributes.md):

```rust
# #![allow(clippy::needless_doctest_main)]
use these_macros_should_be_illegal::{forward_attributes, strutuct};

#[forward_attributes]
#[derive(Debug, PartialEq)]
strutuct! {
    Forwarded { First, Second }
}

fn main() {
    assert_eq!(Forwarded::First, Forwarded::First);
}
```

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    Request
    method: Method { Get, Post, Delete },
    body: String?,
}

let request = Request!(
    method: Method!(Post),
    body: Some("payload".to_owned()),
);

assert!(matches!(request.method, Method::Post));
```

This generates the `Method` enum, the `Request` struct, and same-name
constructor macros.

## Declaration shapes

The body shape decides what gets generated:

- leading `name: Type` members form a struct;
- one parenthesized product containing at least two types forms a tuple struct;
- anything else forms an enum.

Nested `{ ... }` bodies use the same rules recursively.

The root body may either follow its name directly or use ordinary declaration
braces. Optional `struct` and `enum` keywords make the input look more like Rust;
shape inference still decides the output, and a mismatched keyword is diagnosed:

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    pub struct Request {
        method: enum Method { Get, Post, Delete },
        body: String?,
    }
}
```

Standard Rust visibility forms and `priv` are accepted before generated
declarations and named fields. `priv` is the explicit private counterpart to
the macro's default-public behavior.

Nested declarations can also appear inside ordinary generic types. The generated
type is hoisted as usual, while the surrounding container stays untouched:

```rust
use these_macros_should_be_illegal::strutuct;

struct Delimited<T>(T);

strutuct! {
    Arguments
    content: Delimited<Option<ArgumentListContent {
        Empty,
        Values(Vec<String>),
    }>>,
}

let arguments = Arguments!(
    content: Delimited(Some(ArgumentListContent!(Empty))),
);

assert!(matches!(
    arguments.content,
    Delimited(Some(ArgumentListContent::Empty)),
));
```

This works recursively through generic arguments, tuples, arrays, references,
and other grouped type syntax. A declaration inside a type macro invocation is
left to that macro instead of being hoisted by `strutuct!`.

Keywords, visibility, attributes, and local configuration work inside generic
arguments as well. For example, `Vec<priv enum Choice { A, B }>` hoists a private
`Choice` enum and leaves the field type as `Vec<Choice>`.

## Enum grammar

Parentheses refer to terminal payload types that already exist. Vertical bars
declare the generated payload type's name.

```text
Name |Type| { ... }  defines Type and emits Name(Type)
Name |Type|          defines unit Type and emits Name(Type)
Name { ... }         defines ParentName and emits Name(ParentName)
|Type| { ... }       defines Type and emits TypeParent(Type)
|Type|               defines unit Type and emits TypeParent(Type)
Name(Type)           uses Type and stays Name(Type)
Name                 stays Name
(Type)               uses Type and becomes TypeParent(Type)
```

A bare `|Type|` generates the nominal unit struct `pub struct Type;`: this is
the `()` case, not the uninhabited `!` case. The same spelling works as a struct
field type:

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    State
    marker: |EmptyState|,
}

let state = State!(marker: EmptyState);
```

## Products and existing Rust types

By default, a multi-field enum variant carries one tuple product:

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    Value
    Pair(String, u8)
}

let value = Value!(Pair("answer".to_owned(), 42));
assert!(matches!(value, Value::Pair((_, 42))));
```

Disable that lowering for a declaration family when ordinary Rust variants are
more useful:

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    #[strutuct(product_variants = false)]
    Value
    Pair(String, String)
    Span { start: usize, end: usize }
    #[strutuct(product_variants = true)]
    StillPacked(String, String)
}
```

The same configuration attribute before a variant overrides the family
setting for that variant.

## Configuration and visibility

Every option can be set on the root declaration or locally before one field or
variant branch:

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    #[strutuct(public = false, reverse_concat = true)]
    struct Syntax {
        hidden: enum Hidden { A, B },
        #[strutuct(public = true)]
        pub visible: pub enum Visible { A, B },
    }
}
```

The available options are:

- `product_variants = true | false` selects packed products versus ordinary
  multi-field enum variants;
- `public = true | false` selects the default visibility for that declaration
  branch; `false` restores Rust's ordinary private-by-default behavior;
- `reverse_concat = true | false` reverses every automatically concatenated
  name in that branch. For example, `ParentName` becomes `NameParent`, while
  explicit names between `|...|` stay unchanged.

An explicit `pub`, restricted `pub(...)`, or `priv` wins for that individual
declaration or field. Local configuration is inherited by generated declarations
below that object, while siblings retain their surrounding configuration.

## Option, box, attributes, and derives

Postfix `T?` and `T*` become `Option<T>` and `Box<T>`. Wrapped edges stop
constructor-macro recursion, which makes `T*` useful for recursive families.

All ordinary root declaration attributes propagate to generated nested
declarations. This includes `derive`, `cfg`, `cfg_attr`, and third-party
attributes. Ordinary field attributes remain on their fields. A `derive`
before an inline generated struct field adds traits for that declaration and
every generated declaration below it:

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    Token
    #[derive(Copy, Hash)]
    kind: LiteralKind { String, Integer },
}
```

Here `LiteralKind` retains the four inherited traits and additionally derives
`Copy` and `Hash`. Repeated traits are deduplicated. For `Default`, select an
enum's default variant with Rust's ordinary `#[default]` attribute.

Put documentation and other root attributes inside the macro, immediately
before the generated type's name:

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    /// A literal token recognized by the parser.
    Literal
    String
    Integer
}
```

A doc comment written above a bare `strutuct!` belongs to the macro invocation
itself. Put [`forward_attributes`](forward-attributes.md) before the invocation's
other active attributes when those attributes should configure and propagate
through the generated family.

## Constructor paths

Generated enum macros consume nested path segments recursively. Conceptually:

```text
A!(B::C::D(value))
```

becomes:

```text
A::B(AB::C(ABC::D(value)))
```

A generated struct or tuple ends the path and accepts its corresponding fields.
