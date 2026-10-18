# `strutuct!`

API reference on docs.rs: [`strutuct!`](https://docs.rs/these-macros-should-be-illegal/latest/these_macros_should_be_illegal/macro.strutuct.html), [`emmun!`](https://docs.rs/these-macros-should-be-illegal/latest/these_macros_should_be_illegal/macro.emmun.html).

`strutuct!` keeps small related types where they are used, then hoists them into
ordinary Rust declarations in dependency order. Generated declarations and
fields are private unless a visibility says otherwise. `emmun!` is an exact
alias with the same syntax.

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">
<div class="highlight-comparison-pane">

```rust
strutuct! {
    Request
    method: Method { Get, Post },
    body: String?,
}
```
</div>

<div class="highlight-comparison-pane">

```rust,ignore
enum RequestMethod {
    Get,
    Post,
}

struct Request {
    method: RequestMethod,
    body: Option<String>,
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
enum Forwarded {
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
enum RequestMethod {
    Get,
    Post,
    Delete,
}

struct Request {
    method: RequestMethod,
    body: Option<String>,
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

A parenthesized body produces a tuple struct; a nonempty braced body produces a
named-field struct or an enum:

```rust
strutuct! {
    struct Record {
        value: u8,
        flag: bool,
    }
}

strutuct! {
    struct Pair(u8, u16)
}

strutuct! {
    enum Choice {
        First,
        Second,
    }
}
```

A braced body beginning with `name: Type` is a named-field struct. Every other
nonempty braced body is an enum. A parenthesized body is a tuple struct.

The `struct` and `enum` keywords are optional checks on the inferred shape.
They do not force a body to have that shape.

#### Restrictions

A root declaration cannot be empty or unit-like:

```rust,compile_fail
strutuct! {
    Empty {}
}
```

A written keyword must agree with the inferred body:

```rust,compile_fail
strutuct! {
    enum NotAnEnum {
        value: u8,
    }
}
```

```rust,compile_fail
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

Nested bodies use the same inference rules as roots. Consequently a field may
declare any shape:

```rust
pub struct Existing;

strutuct! {
    Container {
        record: Record { value: Existing },
        pair: Pair(Existing, Existing),
        choice: Choice { First, Second },
    }
}
```

#### Restrictions

Struct fields require commas. Once a body is inferred as a struct, an
enum-shaped member cannot be mixed into it:

```rust,compile_fail
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
strutuct! {
    Holder {
        marker: Marker {},
    }
}
```

### Tuple structs

A parenthesized body declares a tuple struct, at the root and wherever a type is
expected. Each element is a type, which may itself declare nested types:

```rust
pub struct Left;
pub struct Right;

strutuct! {
    Pair(Left, Right)
}

strutuct! {
    RelativeHolder {
        pair: Pair(Left, Choice { A, B }),
        single: Single(Left),
        empty: Empty(),
    }
}

strutuct! {
    ExactHolder {
        pair: |ExactPair|(Left, Right),
    }
}
```

These declarations produce `Pair`, `RelativeHolderPair`, `RelativeHolderSingle`,
`RelativeHolderEmpty`, and `ExactPair` as tuple structs. Keywords never name a
declaration, so `fn(Left) -> Right` stays a function pointer type.

As in Rust, attributes and visibility before a tuple element belong to that
field, exactly as they do before a named field:

```rust
pub struct Left;

strutuct! {
    struct Fields(
        #[strutuct(pub(crate))] Choice { A, B },
        pub(crate) Left,
        priv pub(crate) Inner(Left),
    )
}
```

Field `0` and `FieldsChoice` are `pub(crate)` from the inherited option, field
`1` is `pub(crate)` as written, and field `2` is private while `FieldsInner`
stays `pub(crate)`.

#### Restrictions

Braces never declare a tuple struct; a parenthesized member inside braces is the
implicit enum-variant form, which accepts exactly one type:

```rust,compile_fail
strutuct! {
    NotATuple {
        (u8, u16)
    }
}
```

### Enums

Enum variants are written as in Rust and stay ordinary Rust variants:

```rust
pub struct Existing;

strutuct! {
    Choice {
        Unit,
        EmptyTuple(),
        ExistingPayload(Existing),
        Product(Existing, Existing),
        EmptyRecord {},
        Record { value: Existing },

        Nested(Nested { First, Second }),
        Pair(Pair(Existing, Existing)),

        (Existing),
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
ExistingPayload(Existing)     tuple-like variant using an existing type
Product(Existing, Existing)   multi-field tuple-like variant
EmptyRecord {}                empty struct-like variant
Record { value: Existing }    struct-like variant

Nested(Nested { ... })        variant carrying a relative enum declaration
Pair(Pair(...))               variant carrying a relative tuple declaration

(Existing)                    existing payload with an inferred variant name
Named |Payload| { ... }       named variant plus exact payload declaration
|Implicit| { ... }            exact payload and inferred variant name
NamedMarker |Marker|          named variant plus exact unit payload
|ImplicitMarker|              exact unit payload and inferred variant name
```

A braced body never contains a bare type, so `Name { ... }` and `Name(...)`
directly inside it are always variants. Types, including new declarations,
appear inside the parentheses of a variant or after a field name.

`(Existing)` uses an already declared type. `|Implicit| { ... }` declares a new
type. Parentheses and vertical bars are therefore not interchangeable.

Commas between enum variants are optional when the next variant is already
structurally recognizable:

```rust
pub struct Existing;

strutuct! {
    Punctuation {
        First
        Second(Existing)
        Third { value: Existing }
    }
}
```

`Name`, `Name()`, and `Name {}` are all legal and distinct, just as they are in
an ordinary Rust enum.

#### Restrictions

Braces after a variant name hold named fields only, as in Rust. Write a nested
enum as a type inside parentheses instead:

```rust,compile_fail
strutuct! {
    NestedWithoutParentheses {
        Nested { First, Second }
    }
}
```

An implicit parenthesized variant accepts exactly one type:

```rust,compile_fail
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
strutuct! {
    NoInferredName {
        ((u8, u16))
    }
}
```

Parenthesized variant fields are followed by nothing. The former contextual
spellings are rejected:

```rust,compile_fail
strutuct! {
    OldSpelling {
        Named(Payload) { First, Second }
    }
}
```

```rust,compile_fail
strutuct! {
    OldImplicit {
        (Payload) { First, Second }
    }
}
```

Rust-style enum discriminants are not part of the `strutuct!` declaration
grammar:

```rust,compile_fail
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
strutuct! {
    Root {
        Relative(Relative {
            Leaf(Leaf { value: u8 })
        })
        Named |Exact| {
            Leaf(Leaf { value: u8 })
        }
    }
}
```

This declares `RootRelative` and `RootRelativeLeaf`. The bars reset the name,
so the other branch declares `Exact` and `ExactLeaf`.

An exact unit declaration is legal only where a nested type is expected:

```rust
strutuct! {
    Holder {
        marker: |Marker|,
    }
}
```

#### Restrictions

The root name is already exact and is written without bars:

```rust,compile_fail
strutuct! {
    |Root|
}
```

An exact name is one identifier, not a path, and generated declarations do not
accept generic parameter, bound, or `where` clauses:

```rust,compile_fail
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
strutuct! {
    Root {
        choice: struct Choice { First, Second },
    }
}
```

Tuple fields have the same two positions: `(priv priv Inner(u8))` is a private
field of the private type `ParentInner`.

#### Variant visibility and shape keywords

As in Rust, a variant has no visibility of its own; it always shares its enum's.
Write the visibility of a generated payload before the payload type:

```rust
strutuct! {
    enum Choice {
        Branch(priv Branch { A, B }),
        priv |Exact| { A, B },
    }
}
```

A leading `|Payload|` member is the payload declaration itself, so its header
may carry the payload's visibility. Before a variant name it is rejected:

```rust,compile_fail
strutuct! {
    VisibilityOnVariants {
        // error: a variant takes no visibility
        priv Branch(Branch { A, B }),
    }
}
```

A shape keyword decorates any payload the member generates, relative or exact,
and is checked against the shape inferred from its body:

```rust
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

1. A parenthesized body is a tuple struct.
2. A braced body with a leading `name: Type` is a named-field struct.
3. Every other nonempty braced body is an enum.

The important neighboring forms are therefore:

```rust
pub struct Existing;

strutuct! { Named { value: Existing } }
strutuct! { Tuple(Existing, Existing) }
strutuct! { Sum { First, Second } }
strutuct! { OneParenthesized { (Existing) } }
strutuct! { NamedVariant { Pair(Existing, Existing) } }
strutuct! { EmptyTupleVariant { Empty() } }
strutuct! { EmptyStructVariant { Empty {} } }
```

`OneParenthesized` is an enum, not a one-field tuple struct.
`NamedVariant` is an enum because `Pair` precedes the parentheses.
`EmptyTupleVariant` and `EmptyStructVariant` are enums with one fieldless
variant; neither is an empty declaration.

This precedence leaves no token sequence with two possible declaration shapes.
Some neighboring spellings intentionally mean different things, and invalid
mixtures fail instead of falling through to another shape.

## Declaration shapes

The body shape decides what gets generated:

- a parenthesized body forms a tuple struct;
- a braced body of leading `name: Type` members forms a struct;
- any other braced body forms an enum.

Nested bodies use the same rules recursively.

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
enum RequestMethod {
    Get,
    Post,
    Delete,
}

pub struct Request {
    method: RequestMethod,
    body: Option<String>,
}
```

</div>

</div>

Standard Rust visibility forms and `priv` are accepted before generated
declarations and named fields, where they mean what they mean in Rust. Written
as an option instead, a visibility is inherited by everything generated below
it; see [configuration and visibility](#configuration-and-visibility).

Declarations appear directly as a field or tuple element type, optionally
wrapped by postfix `?` or `*`. Every other Rust type is copied unchanged,
including its generic arguments, so `Vec<Box<dyn Fn(u8) -> u8>>` keeps its
Rust meaning and `Vec<Choice { A, B }>` is not a declaration. Declare the
element type in its own field or tuple element instead.

## Enum grammar

Variants follow Rust; new types are declared only in type positions. Vertical
bars declare the generated payload type's exact name.

```text
Name                  stays Name
Name(T, U)            stays Name(T, U)
Name { x: T }         stays Name { x: T }
Name(Inner { ... })   defines ParentInner and emits Name(ParentInner)
Name(Inner(T, U))     defines tuple ParentInner and emits Name(ParentInner)
Name |Type| { ... }   defines Type and emits Name(Type)
Name |Type|           defines unit Type and emits Name(Type)
|Type| { ... }        defines Type and emits TypeParent(Type)
|Type|                defines unit Type and emits TypeParent(Type)
(Type)                uses Type and becomes TypeParent(Type)
```

A bare `|Type|` generates the nominal unit struct `pub struct Type;`: this is
the `()` case, not the uninhabited `!` case. Struct fields follow the same
distinction: `field: Type` uses an existing Rust type, `field: Name { ... }`
and `field: Name(...)` generate `ParentName`, and `field: |Type| { ... }`
generates the exact name `Type`. The bar spelling also generates an exact unit field type without a
body:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
strutuct! {
    State
    marker: |EmptyState|,
}

let state = State!(marker: EmptyState);
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
struct EmptyState;

struct State {
    marker: EmptyState,
}

let state = State {
    marker: EmptyState,
};
```

</div>

</div>

## Products and existing Rust types

By default, variants stay exactly as written. With `product_variants`, a
multi-field tuple-like variant carries one tuple product, and a struct-like
variant carries a generated payload struct:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
strutuct! {
    #[strutuct(product_variants = true)]
    enum Value {
        Pair(String, String),
        Span { start: usize, end: usize },
        #[strutuct(product_variants = false)]
        Plain(String, String),
    }
}
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
struct ValueSpan {
    start: usize,
    end: usize,
}

enum Value {
    Pair((String, String)),
    Span(ValueSpan),
    Plain(String, String),
}
```

</div>

</div>

The same configuration attribute before a variant overrides the family
setting for that variant.

## Configuration and visibility

Options are ordinary outer attributes, so they go exactly where Rust accepts an
outer attribute on something `strutuct!` generates:

- before the root declaration;
- before a named or tuple field;
- before a variant.

An option applies to the object it is written on and to every declaration
generated below it, while siblings keep their surrounding configuration. Use
[`forward_attributes`](forward-attributes.md) to write the root options outside
the invocation:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
strutuct! {
    #[strutuct(reverse_concat = true)]
    struct Syntax {
        hidden: enum Hidden { A, B },
        #[strutuct(pub)]
        visible: enum Visible { A, B },
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

- a visibility — `pub`, a restricted `pub(...)`, or `priv` — selects the
  visibility of that object and of every declaration and field generated below
  it; without one, everything generated is private, as in ordinary Rust;
- `inclusions = true | false` generates consuming functions for enum constructor
  paths; it is disabled by default;
- `product_variants = true | false` packs multi-field and struct-like variants
  into one product payload; it is disabled by default;
- `reverse_concat = true | false` reverses every automatically concatenated
  name in that branch. For example, `ParentName` becomes `NameParent`, while
  explicit names between `|...|` stay unchanged.

These are the complete options currently implemented. More can be added without
changing the declaration grammar.

### Inherited and written visibility

The two visibility positions differ only in reach. Written before a declaration
or field, a visibility is that object's own and nothing else's. Written as an
option, it is the default for the whole branch underneath, and any visibility
written inside that branch wins locally without disturbing what it contains:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
strutuct! {
    #[strutuct(pub(crate))]
    struct Request {
        method: Method { Get, Post },
        priv secret: Secret {
            token: String,
        },
    }
}
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
pub(crate) enum RequestMethod {
    Get,
    Post,
}

pub(crate) struct RequestSecret {
    pub(crate) token: String,
}

pub(crate) struct Request {
    pub(crate) method: RequestMethod,
    secret: RequestSecret,
}
```

</div>

</div>

The `priv` before `secret` hides that one field; `RequestSecret` and its `token`
keep the inherited `pub(crate)`. Writing `priv` before the declaration instead,
as in `secret: priv Secret { ... }`, hides the generated struct while leaving
its fields and the field holding it alone.

A type position is not an attribute position in Rust, so options there are
rejected. Configure the enclosing field or variant instead; an explicit
visibility on the field keeps the two apart when they should differ:

```rust,compile_fail
strutuct! {
    struct Holder {
        // error: `#[strutuct(...)]` cannot configure a type
        value: #[strutuct(pub)] enum Choice { A, B },
    }
}
```

```rust
strutuct! {
    struct Holder {
        #[strutuct(pub)]
        priv value: enum Choice { A, B },
    }
}
```

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
enum TokenKind {
    String,
    Integer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
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

```rust
strutuct! {
    #[derive(Debug, Clone, Default)]
    Settings
    #[underive(Default)]
    mode: Mode { Fast, Safe(Level { Low, High }) }?,
}
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
#[derive(Debug, Clone)]
enum SettingsModeLevel {
    Low,
    High,
}

#[derive(Debug, Clone)]
enum SettingsMode {
    Fast,
    Safe(SettingsModeLevel),
}

#[derive(Debug, Clone, Default)]
struct Settings {
    mode: Option<SettingsMode>,
}
```

</div>

</div>

Neither enum has a natural default variant, and `Option<SettingsMode>` defaults
to `None` without one, so only `Settings` keeps `Default`.

Put documentation and other root attributes inside the macro, immediately
before the generated type's name:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
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
enum Literal {
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
strutuct! {
    #[strutuct(inclusions = true)]
    Token {
        Operator(Operator { Not(String), Plus }),
        Keyword(Keyword { Not(String) }),
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
enum TokenOperator {
    Not(String),
    Plus,
}

enum TokenKeyword {
    Not(String),
}

enum Token {
    Operator(TokenOperator),
    Keyword(TokenKeyword),
}

fn TokenOperator(value: TokenOperator) -> Token {
    Token::Operator(value)
}

fn TokenOperatorNot(value: String) -> Token {
    Token::Operator(TokenOperator::Not(value))
}

fn TokenOperatorPlus() -> Token {
    Token::Operator(TokenOperator::Plus)
}

fn TokenKeyword(value: TokenKeyword) -> Token {
    Token::Keyword(value)
}

fn TokenKeywordNot(value: String) -> Token {
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
