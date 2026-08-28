# These Macros Should Be Illegal

[![Crates.io](https://img.shields.io/crates/v/these-macros-should-be-illegal.svg)](https://crates.io/crates/these-macros-should-be-illegal)
[![Documentation](https://docs.rs/these-macros-should-be-illegal/badge.svg)](https://docs.rs/these-macros-should-be-illegal)
[![License](https://img.shields.io/crates/l/these-macros-should-be-illegal.svg)](LICENSE)

A bag of experimental Rust macros for deleting boilerplate and trying syntax
that Rust would, quite reasonably, never accept directly. Useful examples first;
the caveats and implementation yapping are further down.

```sh
cargo add these-macros-should-be-illegal
```

## Keep enum descriptions beside their variants

`discriminated_str` lets you write descriptions exactly where you look for them:
beside the variants. Fixed text, stored text, selected fields, and missing
descriptions can all live in the same enum:

```rust
#[discriminated_str(description)]
enum Failure<'a> {
    Timeout = "the operation timed out",
    Message(String),
    Context { code: u16, text: &'a str } = text,
    Undescribed(u16),
}

assert_eq!(Failure::Timeout.description(), Some("the operation timed out"));
assert_eq!(Failure::Message("broken".into()).description(), Some("broken"));
assert_eq!(
    Failure::Context { code: 500, text: "server error" }.description(),
    Some("server error"),
);
assert_eq!(Failure::Undescribed(7).description(), None);
```

If selecting one field is not enough, use a closure over all the fields:

```rust
#[discriminated_str(display: String)]
enum Name {
    Qualified(String, String) = |module, item| format!("{module}::{item}"),
    Anonymous = "<anonymous>",
}

assert_eq!(
    Name::Qualified("syntax".into(), "Expr".into()).display(),
    "syntax::Expr",
);
```

## Declare small algebraic type families together

`strutuct!` (also available as the wonderfully backward `emmun!`) lets you write
the little types where they are actually used. It pulls them out into normal
Rust declarations and generates constructor macros:

```rust
use these_macros_should_be_illegal::{forward_attributes, strutuct};

#[forward_attributes]
#[derive(Debug, PartialEq)]
strutuct! {
    Status { Ready, Waiting }
}

assert_eq!(format!("{:?}", Status::Ready), "Ready");
```

`forward_attributes` moves every remaining outer attribute behind a `;` inside
the function-like invocation. The invoked macro owns their meaning; `strutuct!`
uses them as inherited attributes and configuration for the generated family.
It must be the first active attribute, because an active attribute placed before
it gets an opportunity to fail first. Macro input stays completely opaque.

```rust
strutuct! {
    pub struct Request {
        method: enum Method { Get, Post, Delete },
        body: String?,
    }
}

let request = Request!(
    method: Method!(Post),
    body: Some("payload".to_owned()),
);
```

That one block creates public `Method` and `Request` types in the right order.
Postfix `T?` and `T*` mean `Option<T>` and `Box<T>`.

Nested enums compose through their generated macros:

```rust
strutuct! {
    Expr
    Unary { (Pref), (Post) }
    (Bin)
    LitStr(String)
    Null
}

let prefix = Expr!(Unary::Pref(1));
let literal = Expr!(LitStr("text".to_owned()));
```

## Stop qualifying the same standard types forever

`qf!` recursively gives common unqualified types their canonical paths. It is
mainly for generated code where relying on the caller's imports would be rude:

```rust
type Messages = qf!(Option<Vec<String>>);
```

That becomes `::core::option::Option<::std::vec::Vec<::std::string::String>>`.
Already qualified names stay untouched, so `application::String` continues to
mean exactly what it says.

## Use deliberately invalid syntax in a side module

`literally_literal_string!` turns `@@"text"` into an owned `String`, because
apparently `.to_owned()` was too much ceremony:

```rust
let greeting: String = literally_literal_string!(@@"hello");
```

## Share match arms that Rust refuses to combine

Rust or-pattern bindings must have one concrete type. `shared_match_arms!`
uses `||` to clone the RHS into independent arms instead:

```rust
let trailing = shared_match_arms! {
    match token {
        TokenTree::Group(token) || TokenTree::Literal(token) =>
            token.trailing_trivia().as_slice(),
    }
};
```

Use `(pattern if guard)` for a separately guarded component. Because this is a
recursive function-like token transform, it also works through `expand!` over
an out-of-line module.

For syntax Rust cannot parse at item level, `expand!` loads an out-of-line module
and applies the selected transformations before Rust sees its body:

```rust
// src/lib.rs
expand!(
    literally_literal_string;
    mod experiments;
);
```

```rust
// src/experiments.rs
pub fn greeting() -> String {
    @@"hello from inadvisable Rust"
}
```

## `discriminated_str` details

The name inside the attribute becomes the method name. Results are borrowed by
default; write `: String` when you want an owned result. `: &str` is also
accepted when you want to be explicit:

```rust
#[discriminated_str(label: String)]
enum OwnedLabel {
    Fixed = "fixed",
    Dynamic(String),
}
```

The rules are intentionally simple:

- `Variant = "text"` supplies a fixed description;
- a sole `String` or `&str` field is inferred automatically;
- `Variant(T, String) = 1` selects a zero-based tuple field;
- `Variant { text: String } = text` selects a named field;
- `Variant(A, B) = |a, b| expression` gets every field in declaration order;
- a variant with no usable description produces `None`.

The generated accessor invokes the closure with borrowed field bindings. Rust
checks its argument count, body, and return type.

The return type works itself out from the whole enum:

- all fixed descriptions produce `const fn -> &'static str`;
- any selected, inferred, or computed description produces `fn -> &str`;
- any missing description wraps the result in `Option`;
- fixed-or-missing enums retain `const fn -> Option<&'static str>`;
- `: String` produces `String` or `Option<String>` instead.

Missing descriptions can use their variant names instead of `None`:

```rust
#[discriminated_str(description = stringify)]
enum State {
    Explicit = "custom spelling",
    Generated,
}

assert_eq!(State::Generated.description(), "Generated");
```

Generics, where clauses, and lifetimes are copied through. Borrowed results live
as long as the borrow of `self`. `cfg` and `cfg_attr` are also copied onto the
generated match arms, otherwise conditional variants would immediately ruin the
fun.

## `strutuct!` details

If the body starts like `name: Type`, it is a struct. One parenthesized product
with at least two elements is a tuple struct. Everything else is an enum.
Nested declarations come out before the declarations using them.

The root body may be wrapped in braces, and optional `struct`/`enum` keywords
are checked against the shape inferred from its members. Standard visibility
and `priv` work on generated declarations and fields. Use
`#[strutuct(public = false)]` for ordinary private-by-default Rust visibility.

Inside an enum, parentheses refer to terminal payload types that already exist.
Vertical bars declare the generated payload type's name. Without a body, a
bar-named declaration generates a nominal unit type (the `()` case, not `!`):

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

The `|Type|` spelling also works in a struct field, such as
`state: |EmptyState|`. The body in `{ ... }` can itself describe a struct,
tuple struct, or enum.
Multi-field tuple-like variants carry one tuple product, so `Pair(X, Y)` emits
`Pair((X, Y))`. Struct-like syntax similarly creates a product type by default:

```rust
strutuct! {
    Value
    String { Raw, Escaped }
    Span { start: usize, end: usize }
}
```

This creates `ValueString` and `ValueSpan`, while the variants remain pleasantly
named `String(ValueString)` and `Span(ValueSpan)`.

If ordinary Rust multi-field variants are useful for one declaration family,
turn product lowering off with an outer configuration attribute:

```rust
strutuct! {
    #[strutuct(product_variants = false)]
    Value
    Pair(String, String)
    Span { start: usize, end: usize }
    #[strutuct(product_variants = true)]
    StillPacked(String, String)
}
```

The same attribute before a variant overrides the declaration-wide setting.

`#[strutuct(reverse_concat = true)]` reverses automatically generated names,
turning `ParentName` into `NameParent`. All configuration options can be applied
to the whole family or locally before one field or variant branch.

Ordinary field attributes are preserved, and every ordinary root declaration
attribute is copied onto generated nested declarations. This keeps
derive helpers such as `#[serde(flatten)]` usable across a nested type family.
A `derive` before an inline struct field adds to the family derives for that
generated type and every declaration nested below it. Repeated traits are
deduplicated. Generated enums use Rust's ordinary `#[default]` variant attribute
to select their default value.

Inline declarations may occur inside ordinary generic containers, so a field
such as `Delimited<Option<ArgumentListContent { Empty, Value(String) }>>`
hoists `ArgumentListContent` and leaves the surrounding type intact. The more
Rust-like `Vec<enum Element { A, B }>` spelling works too.

Each generated type also gets a constructor macro in Rust's conveniently
separate macro namespace. Every enum macro eats one path segment and calls the
next one:

```text
A!(B::C::D(value))
```

folds into:

```text
A::B(AB::C(ABC::D(value)))
```

A nested struct ends the path and accepts named fields. Boxed and optional edges
also stop there, so `T*` can express recursion without making the constructor
macro expand forever. That would be a different kind of illegal.

## The boring but important token rules

Transformations recurse through every token group. Attributes stay opaque, and
excluded macro calls are copied without poking around inside them.

Configure exclusions directly in `expand!`:

```rust
expand!(
    literally_literal_string,
    exclude_macros = (raw_tokens);
    mod experiments;
);
```

Or wrap a configuration-aware item-position macro:

```rust
#[excluded_macros(raw_tokens)]
literally_literal_string! {
    // transformed tokens
}
```

`expand!` currently loads one out-of-line module file. It does not recursively
load external child modules. Cargo may also miss changes made only to files read
by the procedural macro, so rerun the build when necessary.

## About the crate

This is a personal stable-Rust macro laboratory. The APIs are small,
experimental, and allowed to evolve whenever a better crime presents itself.
Closely related macros live in the root crate; experiments that need their own
dependencies or release cycle can move into [`crates/`](crates/).

Development gates:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --no-fail-fast
```

Released through GitHub and crates.io under the [MIT License](LICENSE).
