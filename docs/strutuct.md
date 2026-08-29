# `strutuct!`

`strutuct!` keeps small related types where they are used, then hoists them into
ordinary Rust declarations in dependency order. Generated declarations and
fields are public by default. `emmun!` is an exact alias with the same syntax.

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust,ignore
strutuct! {
    Request
    method: Method { Get, Post },
    body: String?,
}
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
pub enum RequestMethod {
    Get,
    Post,
}

pub struct Request {
    pub method: RequestMethod,
    pub body: Option<String>,
}

// RequestMethod!(...) and Request!(...)
// constructor macros are generated too.
```

</div>

</div>

Outer attributes can configure the complete generated family through
[`forward_attributes`](forward-attributes.md):

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
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

</div>

<div class="highlight-comparison-pane">

```rust,ignore
#[derive(Debug, PartialEq)]
pub enum Forwarded {
    First,
    Second,
}

fn main() {
    assert_eq!(Forwarded::First, Forwarded::First);
}
```

</div>

</div>

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    Request
    method: Method { Get, Post, Delete },
    body: String?,
}

let request = Request!(
    method: RequestMethod!(Post),
    body: Some("payload".to_owned()),
);

assert!(matches!(request.method, RequestMethod::Post));
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
pub enum RequestMethod {
    Get,
    Post,
    Delete,
}

pub struct Request {
    pub method: RequestMethod,
    pub body: Option<String>,
}

let request = Request {
    method: RequestMethod::Post,
    body: Some("payload".to_owned()),
};

assert!(matches!(request.method, RequestMethod::Post));
```

</div>

</div>

This generates the `RequestMethod` enum, the `Request` struct, and same-name
constructor macros. Braced `Method { ... }` is a relative generated name;
write `|Method| { ... }` when the generated enum must be named exactly `Method`.

## Declaration shapes

The body shape decides what gets generated:

- leading `name: Type` members form a struct;
- one parenthesized product containing at least two types forms a tuple struct;
- anything else forms an enum.

Nested `{ ... }` bodies use the same rules recursively.

The root body may either follow its name directly or use ordinary declaration
braces. Optional `struct` and `enum` keywords make the input look more like Rust;
shape inference still decides the output, and a mismatched keyword is diagnosed:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    pub struct Request {
        method: enum Method { Get, Post, Delete },
        body: String?,
    }
}
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
pub enum RequestMethod {
    Get,
    Post,
    Delete,
}

pub struct Request {
    pub method: RequestMethod,
    pub body: Option<String>,
}
```

</div>

</div>

Standard Rust visibility forms and `priv` are accepted before generated
declarations and named fields. `priv` is the explicit private counterpart to
the macro's default-public behavior.

Nested declarations can also appear inside ordinary generic types. The generated
type is hoisted as usual, while the surrounding container stays untouched:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::strutuct;

struct Delimited<T>(T);

strutuct! {
    Arguments
    content: Delimited<Option<|ArgumentListContent| {
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

</div>

<div class="highlight-comparison-pane">

```rust,ignore
struct Delimited<T>(T);

pub enum ArgumentListContent {
    Empty,
    Values(Vec<String>),
}

pub struct Arguments {
    pub content: Delimited<Option<ArgumentListContent>>,
}

let arguments = Arguments {
    content: Delimited(Some(ArgumentListContent::Empty)),
};

assert!(matches!(
    arguments.content,
    Delimited(Some(ArgumentListContent::Empty)),
));
```

</div>

</div>

This works recursively through generic arguments, tuples, arrays, references,
and other grouped type syntax. A declaration inside a type macro invocation is
left to that macro instead of being hoisted by `strutuct!`.

Keywords, visibility, attributes, and local configuration work inside generic
arguments as well. For example, `Vec<priv enum |Choice| { A, B }>` hoists a
private `Choice` enum and leaves the field type as `Vec<Choice>`.

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
the `()` case, not the uninhabited `!` case. Struct fields follow the same
distinction: `field: Type` uses an existing Rust type, `field: Name { ... }`
generates `ParentName`, and `field: |Type| { ... }` generates the exact name
`Type`. The bar spelling also generates an exact unit field type without a
body:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    State
    marker: |EmptyState|,
}

let state = State!(marker: EmptyState);
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
pub struct EmptyState;

pub struct State {
    pub marker: EmptyState,
}

let state = State {
    marker: EmptyState,
};
```

</div>

</div>

## Products and existing Rust types

By default, a multi-field enum variant carries one tuple product:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    Value
    Pair(String, u8)
}

let value = Value!(Pair("answer".to_owned(), 42));
assert!(matches!(value, Value::Pair((_, 42))));
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
pub enum Value {
    Pair((String, u8)),
}

let value = Value::Pair(("answer".to_owned(), 42));
assert!(matches!(value, Value::Pair((_, 42))));
```

</div>

</div>

Disable that lowering for a declaration family when ordinary Rust variants are
more useful:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

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

</div>

<div class="highlight-comparison-pane">

```rust,ignore
pub struct ValueSpan {
    pub start: usize,
    pub end: usize,
}

pub enum Value {
    Pair(String, String),
    Span(ValueSpan),
    StillPacked((String, String)),
}
```

</div>

</div>

The same configuration attribute before a variant overrides the family
setting for that variant.

## Configuration and visibility

Every option can be set on the root declaration or locally before one field or
variant branch:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

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

</div>

<div class="highlight-comparison-pane">

```rust,ignore
enum HiddenSyntax {
    A,
    B,
}

pub enum VisibleSyntax {
    A,
    B,
}

struct Syntax {
    hidden: HiddenSyntax,
    pub visible: VisibleSyntax,
}
```

</div>

</div>

The available options are:

- `product_variants = true | false` selects packed products versus ordinary
  multi-field enum variants;
- `public = true | false` selects the default visibility for that declaration
  branch; `false` restores Rust's ordinary private-by-default behavior;
- `reverse_concat = true | false` reverses every automatically concatenated
  name in that branch. For example, `ParentName` becomes `NameParent`, while
  explicit names between `|...|` stay unchanged.

These are the complete options currently implemented. More can be added without
changing the declaration grammar.

An explicit `pub`, restricted `pub(...)`, or `priv` wins for that individual
declaration or field. Local configuration is inherited by generated declarations
below that object, while siblings retain their surrounding configuration.

## Parsing precedence

`strutuct!` is a syntax extension, so its grammar wins whenever its tokens could
also be interpreted as unusually shaped Rust. For example, postfix `?` and `*`
are always the macro's `Option` and `Box` constructors in a type position, and a
braced identifier is always an inline declaration. The syntax is deliberately
chosen to avoid collisions with ordinary real-world Rust, but ambiguous input is
resolved consistently in favor of `strutuct!` rather than guessed from intent.

## Generated-code lint guards

Generated declarations carry `#[allow(dead_code)]`, because a complete algebraic
hierarchy commonly contains types, fields, or variants that one consumer does
not use. Generated constructor macros similarly carry
`#[allow(unused_macros)]`. Every generated item already has synthetic
documentation, and no local variable bindings are generated, so
`missing_docs` and `unused_variables` do not need suppression. User attributes
are emitted after the generated declaration guard, so a local
`#[deny(dead_code)]` can opt a branch back into checking.

## Option, box, attributes, and derives

Postfix `T?` and `T*` become `Option<T>` and `Box<T>`. Wrapped edges stop
constructor-macro recursion, which makes `T*` useful for recursive families.

Ordinary root declaration attributes propagate to generated nested declarations,
with one deliberate exception: documentation stays on exactly the declaration
or field where it was written. This prevents one root comment from becoming the
documentation for every generated subtype.

`derive` attributes are merged and deduplicated. `cfg`, `cfg_attr`, and
third-party attributes are copied verbatim; `strutuct!` does not guess whether
an arbitrary attribute supports every generated item shape, so that attribute
or Rust remains responsible for diagnosing an incompatible target. Ordinary
field and variant attributes remain local to those fields and variants. An
attribute written inside a type position, such as
`field: #[some_attribute] Child { ... }`, belongs to the generated `ParentChild`
declaration and propagates through that generated branch. Conditional-compilation
attributes also guard the declaration's constructor macro, keeping cfg-exclusive
families exclusive in both Rust's type and macro namespaces.

A `derive` before an inline generated struct field adds traits for that
declaration and every generated declaration below it:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    Token
    #[derive(Copy, Hash)]
    kind: Kind { String, Integer },
}
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub enum TokenKind {
    String,
    Integer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
}
```

</div>

</div>

Here `TokenKind` retains the four inherited traits and additionally derives
`Copy` and `Hash`. Repeated traits are deduplicated. For `Default`, select an
enum's default variant with Rust's ordinary `#[default]` attribute.

`#[underive(Trait, ...)]` performs the opposite branch-local operation. It
removes matching paths after inherited and local derives are merged, ignores
paths that were absent, and propagates the reduced derive list further down:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust,ignore
strutuct! {
    #[derive(Debug, Clone, PartialEq)]
    Syntax
    #[underive(Clone)]
    node: Node {
        leaf: Leaf { Text(String) },
    },
}
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
#[derive(Debug, PartialEq)]
pub enum SyntaxNodeLeaf {
    Text(String),
}

#[derive(Debug, PartialEq)]
pub struct SyntaxNode {
    pub leaf: SyntaxNodeLeaf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Syntax {
    pub node: SyntaxNode,
}
```

</div>

</div>

Put documentation and other root attributes inside the macro, immediately
before the generated type's name:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    /// A literal token recognized by the parser.
    Literal
    String
    Integer
}
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
/// A literal token recognized by the parser.
pub enum Literal {
    String,
    Integer,
}
```

</div>

</div>

A doc comment written above a bare `strutuct!` belongs to the macro invocation
itself. Put [`forward_attributes`](forward-attributes.md) before the invocation's
other active attributes when those attributes should configure and propagate
through the generated family.

## Constructor paths

Generated enum macros consume nested path segments recursively. Conceptually:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```text
A!(B::C::D(value))
```

</div>

<div class="highlight-comparison-pane">

```text
A::B(AB::C(ABC::D(value)))
```

</div>

</div>

A generated struct or tuple ends the path and accepts its corresponding fields.
