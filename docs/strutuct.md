# `strutuct!`

`strutuct!` keeps small related types where they are used, then hoists them into
ordinary public Rust declarations in dependency order.

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

## Option, box, attributes, and derives

Postfix `T?` and `T*` become `Option<T>` and `Box<T>`. Wrapped edges stop
constructor-macro recursion, which makes `T*` useful for recursive families.

Root `derive`, `cfg`, and `cfg_attr` attributes propagate to generated nested
declarations. Ordinary field attributes remain on their fields. A `derive`
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

A doc comment written above `strutuct!` belongs to the macro invocation itself.
Function-like procedural macros receive only the tokens inside their delimiters,
so `strutuct!` cannot recover or forward that outer comment.

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
