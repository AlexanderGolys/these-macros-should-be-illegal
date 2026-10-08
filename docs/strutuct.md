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

## Declaration forms

The forms below are the complete set of nominal declarations introduced by
`strutuct!` itself. Ordinary Rust types may still be used as payloads, but they
do not change how a declaration body is classified.

### Root declarations

A nonempty body produces a named-field struct, a tuple struct, or an enum. The
body shape selects which one:

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    struct Record {
        value: u8,
        flag: bool,
    }
}

strutuct! {
    struct Pair {
        (u8, u16)
    }
}

strutuct! {
    enum Choice {
        First,
        Second,
    }
}
```

A body beginning with `name: Type` is a named-field struct. A body consisting
of exactly one parenthesized list of two or more types is a tuple struct. Every
other nonempty body is an enum.

The `struct` and `enum` keywords are optional checks on the inferred shape.
They do not force a body to have that shape.

#### Restrictions

A root declaration cannot be empty or unit-like:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    Empty {}
}
```

A written keyword must agree with the inferred body:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    enum NotAnEnum {
        value: u8,
    }
}
```

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    struct NotAStruct {
        First,
        Second,
    }
}
```

### Named-field structs

Once the first member has the form `name: Type`, every member must be a named
field. A field type may itself declare a relative type, an exactly named type,
or an exactly named unit struct:

```rust
use these_macros_should_be_illegal::strutuct;

pub struct Existing;

strutuct! {
    Structs {
        plain: Existing,
        relative: Relative {
            value: Existing,
        },
        exact: |Exact| {
            value: Existing,
        },
        marker: |Marker|,
    }
}
```

This declares `StructsRelative`, `Exact`, and the unit struct `Marker` in
addition to `Structs`.

Nested braced bodies use the same inference rules as roots. Consequently a
field may declare any non-unit shape:

```rust
use these_macros_should_be_illegal::strutuct;

pub struct Existing;

strutuct! {
    Container {
        record: Record { value: Existing },
        pair: Pair { (Existing, Existing) },
        choice: Choice { First, Second },
    }
}
```

#### Restrictions

Struct fields require commas. Once a body is inferred as a struct, an
enum-shaped member cannot be mixed into it:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    Mixed {
        value: u8,
        Variant,
    }
}
```

A relative name requires a nonempty braced body. Only an exact `|Name|` may
declare a unit struct:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    Holder {
        marker: Marker {},
    }
}
```

### Tuple structs

A tuple declaration is one complete parenthesized product containing at least
two types. The rule is identical at the root and inside another declaration:

```rust
use these_macros_should_be_illegal::strutuct;

pub struct Left;
pub struct Right;

strutuct! {
    Pair {
        (Left, Right)
    }
}

strutuct! {
    RelativeHolder {
        pair: Pair { (Left, Right) },
    }
}

strutuct! {
    ExactHolder {
        pair: |ExactPair| { (Left, Right) },
    }
}
```

These declarations produce `Pair`, `RelativeHolderPair`, and `ExactPair` as
tuple structs.

#### Restrictions

There are no generated zero-field or one-field tuple structs. A single
parenthesized type is instead the implicit enum-variant form:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

pub struct Existing;

strutuct! {
    struct OneField {
        (Existing)
    }
}
```

An empty product is not a declaration body:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    ZeroFields {
        ()
    }
}
```

The product must be the entire body. Adding another member makes the body an
enum body, where an implicit parenthesized variant may contain only one type:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    PairAndMore {
        (u8, u16),
        More,
    }
}
```

### Enums

An enum accepts unit variants, tuple-like variants, existing implicit payload
types, and generated payload declarations:

```rust
use these_macros_should_be_illegal::strutuct;

pub struct Existing;

strutuct! {
    Choice {
        Unit,
        EmptyTuple(),
        ExistingPayload(Existing),
        Product(Existing, Existing),
        (Existing),

        Record { value: Existing },
        Pair { (Existing, Existing) },
        Nested { First, Second },

        Named |Payload| { First, Second },
        |Implicit| { First, Second },
        NamedMarker |Marker|,
        |ImplicitMarker|,
    }
}
```

The forms mean:

```text
Unit                          unit variant
EmptyTuple()                  empty tuple-like variant
ExistingPayload(Existing)     named variant using an existing type
Product(Existing, Existing)   named multi-field variant
(Existing)                    existing payload with an inferred variant name

Record { value: Existing }    variant plus relative named-field payload
Pair { (Existing, Existing) } variant plus relative tuple payload
Nested { First, Second }      variant plus relative enum payload

Named |Payload| { ... }       named variant plus exact payload declaration
|Implicit| { ... }            exact payload and inferred variant name
NamedMarker |Marker|          named variant plus exact unit payload
|ImplicitMarker|              exact unit payload and inferred variant name
```

`(Existing)` uses an already declared type. `|Implicit| { ... }` declares a new
type. Parentheses and vertical bars are therefore not interchangeable.

Commas between enum variants are optional when the next variant is already
structurally recognizable:

```rust
use these_macros_should_be_illegal::strutuct;

pub struct Existing;

strutuct! {
    Punctuation {
        First
        Second(Existing)
        Third { A B }
    }
}
```

`Name` and `Name()` are both legal and distinct, just as they are in an
ordinary Rust enum.

#### Restrictions

An implicit parenthesized variant accepts exactly one type:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    BadImplicit {
        (u8, u16)
        Unit
    }
}
```

Its type must also have a final path segment from which a variant name can be
formed. Name a non-path payload explicitly:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    NoInferredName {
        ((u8, u16))
    }
}
```

Parentheses never declare a generated payload type. The former contextual
spellings are rejected:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    OldSpelling {
        Named(Payload) { First, Second }
    }
}
```

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    OldImplicit {
        (Payload) { First, Second }
    }
}
```

Braced payload declarations must be nonempty:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    EmptyPayload {
        Payload {}
    }
}
```

Rust-style enum discriminants are not part of the `strutuct!` declaration
grammar:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    Discriminants {
        First = 1,
        Second = 2,
    }
}
```

### Relative and exact names

An unbarred nested declaration name is relative to its generated parent.
Nesting continues to concatenate names:

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    Root {
        Relative {
            Leaf { value: u8 }
        }
        Named |Exact| {
            Leaf { value: u8 }
        }
    }
}
```

This declares `RootRelative` and `RootRelativeLeaf`. The bars reset the name,
so the other branch declares `Exact` and `ExactLeaf`.

An exact unit declaration is legal only where a nested type is expected:

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    Holder {
        marker: |Marker|,
    }
}
```

#### Restrictions

The root name is already exact and is written without bars:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    |Root|
}
```

An exact name is one identifier, not a path, and generated declarations do not
accept generic parameter, bound, or `where` clauses:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    Generic<T> {
        value: T,
    }
}
```

### Attributes, visibility, and shape keywords

Attributes and visibility may precede roots, named fields, and inline generated
declarations. A declaration keyword may be added before an inline declaration
to validate its inferred shape:

```rust
use these_macros_should_be_illegal::strutuct;

#[derive(Debug)]
pub struct Existing;

strutuct! {
    #[derive(Debug)]
    pub struct Root {
        #[doc = "A private generated branch."]
        pub branch: #[derive(Clone)] priv enum Branch {
            First,
            Second,
        },
        marker: struct |Marker|,
        existing: Existing,
    }
}
```

Here `pub` before `branch` is the visibility of the generated Rust field;
`priv` before `enum Branch` is the visibility of `RootBranch`. Attributes before
the field and attributes before its generated type likewise have different
targets.

The keyword is still only a validator:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    Root {
        choice: struct Choice { First, Second },
    }
}
```

Tuple declaration components are currently type expressions only: unlike named
fields, they do not have DSL positions for individual field attributes or
visibility.

#### Member visibility and shape keywords

A visibility before an enum member applies to the payload type that member
generates. Rust variants always share their enum's visibility, so a visibility
on a member that generates nothing is rejected rather than ignored:

```rust,compile_fail
use these_macros_should_be_illegal::strutuct;

strutuct! {
    VisibilityOnPlainVariants {
        // error: visibility applies to a generated payload type
        pub Unit,
        priv Existing(u8),
    }
}
```

A shape keyword decorates any payload the member generates, relative or exact,
and is checked against the shape inferred from its body:

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    ExactPayloadWithKeyword {
        enum |Payload| { First, Second },
        struct Named |Marker|,
    }
}

let exact = ExactPayloadWithKeyword::PayloadExactPayloadWithKeyword(Payload::First);
let named = ExactPayloadWithKeyword::Named(Marker);
assert!(matches!(exact, ExactPayloadWithKeyword::PayloadExactPayloadWithKeyword(Payload::First)));
assert!(matches!(named, ExactPayloadWithKeyword::Named(Marker)));
```

### Disambiguation

The parser resolves every declaration body in this order:

1. A leading `name: Type` makes the complete body a named-field struct.
2. Otherwise, one complete `(A, B, ...)` body with at least two types makes a
   tuple struct.
3. Every other nonempty body is an enum.

The important neighboring forms are therefore:

```rust
use these_macros_should_be_illegal::strutuct;

pub struct Existing;

strutuct! { Named { value: Existing } }
strutuct! { Tuple { (Existing, Existing) } }
strutuct! { Sum { First, Second } }
strutuct! { OneParenthesized { (Existing) } }
strutuct! { NamedVariant { Pair(Existing, Existing) } }
strutuct! { EmptyTupleVariant { Empty() } }
```

`OneParenthesized` is an enum, not a one-field tuple struct.
`NamedVariant` is an enum because `Pair` precedes the parentheses.
`EmptyTupleVariant` is an enum with one fieldless tuple-like variant; it is not
an empty declaration.

This precedence leaves no token sequence with two possible declaration shapes.
Some neighboring spellings intentionally mean different things, and invalid
mixtures fail instead of falling through to another shape.

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

- `inclusions = true | false` generates consuming functions for enum constructor
  paths; it is disabled by default;
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
documentation, so `missing_docs` does not need suppression. Every binding
generated for an inclusion function is consumed, so `unused_variables` does
not need suppression either. User attributes
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

## Enum inclusions

Set `inclusions = true` to generate the canonical consuming injection for every
constructor occurrence in an enum tree:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    #[strutuct(inclusions = true)]
    Token {
        Operator { Not(String), Plus },
        Keyword { Not(String) },
    }
}

let token = TokenOperatorNot("!".to_owned());
assert!(matches!(
    token,
    Token::Operator(TokenOperator::Not(value)) if value == "!"
));
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
pub enum TokenOperator {
    Not(String),
    Plus,
}

pub enum TokenKeyword {
    Not(String),
}

pub enum Token {
    Operator(TokenOperator),
    Keyword(TokenKeyword),
}

pub fn TokenOperator(value: TokenOperator) -> Token {
    Token::Operator(value)
}

pub fn TokenOperatorNot(value: String) -> Token {
    Token::Operator(TokenOperator::Not(value))
}

pub fn TokenOperatorPlus() -> Token {
    Token::Operator(TokenOperator::Plus)
}

pub fn TokenKeyword(value: TokenKeyword) -> Token {
    Token::Keyword(value)
}

pub fn TokenKeywordNot(value: String) -> Token {
    Token::Keyword(TokenKeyword::Not(value))
}
```

</div>

</div>

Names encode the complete constructor occurrence, not merely the endpoint type.
Consequently, equal leaf types in distinct branches produce distinct functions
such as `TokenOperatorNot` and `TokenKeywordNot`. Unit variants produce
zero-argument functions; other inclusions consume their payload and return the
root enum.

Joining proceeds only through nested enums. Products, generic containers, and
postfix `?` or `*` wrappers are boundaries; an enum below such a boundary keeps
its own independently generated inclusions instead. A branch-local
`#[strutuct(inclusions = false)]` stops that branch, while a local `true` can
enable one branch when the surrounding declaration leaves the feature off.

For a chain of depth `N`, the macro emits `N` functions. Their composed bodies
have total worst-case size `O(N²)`; the macro never enumerates the possible
factorizations of a path. Internally this is a bottom-up fold followed by the
iterated join of homogeneous coproduct layers.

Tuple and unit structs also introduce value constructors. If an automatically
named inclusion would occupy the same value-namespace name, `strutuct!` reports
the collision. Give the payload a distinct exact name with `Name |Payload| { ... }`,
disable inclusions for that branch, or use a non-tuple payload.
