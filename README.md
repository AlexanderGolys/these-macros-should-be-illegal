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

Short names are exported directly and can be imported without renaming:

| Full name | Short name |
| --- | --- |
| `literally_literal_string!` | `lls!` |
| `discriminated_str` | `disctr` |
| `shared_match_arms!` | `sma!` |

## Give an object function-like syntax without erasing its type

`callable` aliases one method of a user-owned trait to the crate's structural
calling convention. `make_fn!` then creates both a local value and its
same-name invocation macro:

```rust
use these_macros_should_be_illegal::{callable, make_fn};

#[callable(apply)]
trait Action {
    fn apply(&self, point: usize) -> usize;
}

struct Shift(usize);

impl Action for Shift {
    fn apply(&self, point: usize) -> usize {
        point + self.0
    }
}

make_fn!(sigma = Shift(3));

assert_eq!(sigma!(2), 5);
```

`sigma` remains a `Shift`, so its ordinary structural methods remain available;
only `sigma!` occupies the macro namespace.

## Generate small enum methods beside their variants

`enum_fn` lets you write match-arm expressions exactly where you look for them:
beside the variants. Fixed values, closures, and missing arms
can all live in the same enum:

```rust
#[enum_fn(description -> Option<&str>, missing = None)]
enum Failure<'a> {
    Timeout = Some("the operation timed out"),
    Message(String) = |value| Some(value.as_str()),
    Context { code: u16, text: &'a str } = |code, text| { let _ = code; Some(*text) },
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

Closures can compute a value from all the fields:

```rust
#[enum_fn(display -> String)]
enum Name {
    Qualified(String, String) = |module, item| format!("{module}::{item}"),
    Anonymous = || String::from("<anonymous>"),
}

assert_eq!(
    Name::Qualified("syntax".into(), "Expr".into()).display(),
    "syntax::Expr",
);
```

## Write the constant part of a field once

`forall` takes the beginning of one member declaration — outer attributes, a
visibility, the shared types, or one group of shared named fields, in any
order — and repeats it on every field or variant:

```rust
#[forall(pub(crate))]
pub struct Pixel(u8, u8, u8);

#[forall(pub, f32)]
struct Point(x, y);

#[forall(u16, String)]
enum Response {
    Redirect,
    Error(&'static str),
}

#[forall({ code: u16 })]
enum Request {
    Read,
    Write { payload: u8 },
}
```

`Point` becomes a braced struct whose `x` and `y` are both `pub f32`, since a
tuple struct given one shared type has its positions read as field names.
`Response::Redirect` gains `(u16, String)` and `Response::Error` gains them
after its own field, while every variant of `Request` gains the named `code`.
A member keeps the visibility it declares itself, and a created named field is
documented unless the declaration documents it. A member marked
`#[forall(skip)]` takes none of the prefix, and `#[forall(skip(String))]` keeps
out only the fragments it names, across every stacked `forall`.

## Read a struct as the product over its field names

A struct is a product over its field names, and when every field holds one type
`X` that product is an exponential: `S = X^E = Hom(E, X)`. `Log` derives that
index set `E`, which makes the evaluation map `E x S -> X` an ordinary indexing
operator:

```rust
#[derive(Log)]
#[allow(non_snake_case)]
struct BaseGroups {
    Keyword: Style,
    Function: Style,
    Bg: Style,
    Text: Style,
}

let mut groups = BaseGroups { /* ... */ };
groups[LogBaseGroups::Bg] = Style::default();

assert_eq!(groups[LogBaseGroups::Keyword], groups.Keyword);
```

The variants are spelled exactly as the fields, and the enum is named `Log` and
the struct's name unless its `log` attribute declares it:
`#[log(pub enum Axis)]`. Fields holding different types are a genuine product
with no single `X` to index out of, so they derive the enum alone. Attribute
macros run before derives, so with
[`forall`](#write-the-constant-part-of-a-field-once) above it the whole family
costs one line each:

```rust
#[forall(pub, Style)]
#[derive(Log)]
#[log(pub Group)]
struct Theme(Keyword, Function, Bg);
```

## Give variants genuine string discriminants

`discriminated_str` assigns every variant one unique literal and generates a
forward method plus a same-name delegating constructor macro:

```rust
#[discriminated_str(name)]
enum Token {
    Ident(String) = "ident",
    End = "end",
}

let token = Token!("ident", String::from("value"));
assert_eq!(token.name(), "ident");
assert!(matches!(Token!("end"), Token::End));
```

The literal chooses the variant during expansion; the remaining expressions
are passed directly to its ordinary Rust constructor.

## Write an operator once, use it with any ownership

`overload_op` takes the impl written for references and generates the owned
forms and the augmented assignment; `complete_ops` adds the operators that
follow from existing ones, such as subtraction from addition and negation:

```rust
#[derive(Debug, PartialEq)]
#[complete_ops(Sub)]
struct Vector(f64);

#[overload_op]
impl Add<&Vector> for &Vector {
    type Output = Vector;

    fn add(self, rhs: &Vector) -> Vector {
        Vector(self.0 + rhs.0)
    }
}

#[overload_op]
impl Neg for &Vector {
    type Output = Vector;

    fn neg(self) -> Vector {
        Vector(-self.0)
    }
}

let mut total = Vector(1.0) + &Vector(2.0);
total += Vector(3.0);
assert_eq!(total - Vector(1.0), Vector(5.0));
```

Nothing is cloned: every generated impl delegates to the borrowed one.

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

fn main() {
    assert_eq!(format!("{:?}", Status::Ready), "Ready");
}
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
    method: RequestMethod!(Post),
    body: Some("payload".to_owned()),
);
```

That one block creates public `RequestMethod` and `Request` types in the right
order. A braced nested name is relative to its generated parent; use
`method: |Method| { ... }` when the nested type must be named exactly `Method`.
Postfix `T?` and `T*` mean `Option<T>` and `Box<T>`.

Nested enums compose through their generated macros:

```rust
pub struct Pref(pub u8);
pub struct Post(pub u8);
pub struct Bin(pub u8);

strutuct! {
    enum Expr {
        Unary(Unary { (Pref), (Post) }),
        (Bin),
        LitStr(String),
        Null,
    }
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

## `enum_fn` details

Write `#[enum_fn(const name -> Type, missing = expression)]` on an enum.

| Argument | Accepted value | Default |
| --- | --- | --- |
| `name` | Generated method's identifier | Required |
| `Type` | Exact Rust return type after `->` | Required |
| `missing` | Expression used by the wildcard arm | `::core::panic!()` |

Methods borrow `&self` and are private unless a visibility precedes the name,
for example `#[enum_fn(pub const label -> String)]`. Methods are runtime by
default; put `const` after visibility to request a `const fn`. A trailing comma
is accepted.
Attributes placed before the signature inside the arguments apply to the
generated method, such as `#[enum_fn(#[doc = "Returns the label."] label -> String)]`.
Without a method doc attribute, only the generated method gets
`#[allow(missing_docs)]`; enum-level docs stay on the enum.
Types and values are used as written; no conversion, cloning, or optional
wrapping is implicit. Without `const`, any Rust expression is accepted,
including calls such as `f(2)`. With `const`, Rust checks that every variant
expression and the fallback are const-compatible. Const methods cannot use
lambdas.

Variant values are Rust expressions or lambdas:

- `Variant = expression` supplies a fixed result, independent of payload fields;
- `Variant(T, String) = |x, text| expression` borrows one argument per field;
- `Variant { code: T } = |code| expression` requires the field's own name;
- unit variants and empty products use `|| expression` for runtime computation;
- a variant without a value reaches `_ => missing`.

Tuple argument names are arbitrary; named arguments follow field declaration
order. The macro checks arity and names, while Rust checks types. A single
field containing a tuple is still one field. Borrowed results can live as long
as the borrow of `self`.

Use `#[enum_fn(const code -> u8)]` when callers need to use the method in a
constant. Use `|| String::from("fixed")` for an owned runtime value on a unit
variant.
Fixed-result methods preserve elided return references such as `&str`; lambda
methods use an explicit borrow lifetime so they can return references to fields.

The wildcard is always emitted, so `non_exhaustive` works in either attribute
order. A missing value defaults to `panic!()`; a custom fallback can be a value,
a runtime expression, or a custom panic:

```rust
#[enum_fn(const code -> u8, missing = 255)]
enum Status {
    Ready = 1,
    Unknown,
}

const UNKNOWN: u8 = Status::Unknown.code();
assert_eq!(UNKNOWN, 255);
```

For optional results, explicitly declare `Option<T>`, write `Some(...)` for
present values (including lambda results), and set `missing = None`.
Generics, where clauses, lifetimes, and conditional compilation are preserved.
See the [full reference](docs/enum_fn.md) for examples.

## `strutuct!` details

A parenthesized body declares a tuple struct. A braced body starting like
`name: Type` is a struct, and any other braced body is an enum. Nested
declarations come out before the declarations using them.

The root body may be wrapped in braces, and optional `struct`/`enum` keywords
are checked against the shape inferred from its members. Everything generated
is private unless a visibility says otherwise, exactly as in ordinary Rust.

Enum variants are written exactly as in Rust and stay ordinary variants:

```rust
Name                 unit variant
Name(), Name(T, U)   tuple-like variant
Name {}, Name { x: T } struct-like variant
```

New types are declared only where a type is expected: in a named or tuple
field, or in a variant's parentheses. There, `Name { ... }` declares a
struct or enum and `Name(...)` declares a tuple struct, both named relative to
their generated parent. Bars request an exact name instead, and a bar-named
declaration without a body is a nominal unit type (the `()` case, not `!`):

```rust
field: Name { ... }    defines struct or enum ParentName
field: Name(T, U)      defines tuple struct ParentName
field: |Type| { ... }  defines Type
field: |Type|(T, U)    defines tuple struct Type
field: |Type|          defines unit struct Type
Name(Inner { ... })    defines ParentInner and emits Name(ParentInner)
```

A braced body never contains a bare type, so `Name { ... }` and `Name(...)`
directly inside one are always variants. Keywords never name a declaration, so
`fn(u8) -> u8` stays a function pointer.

Enums also accept three payload shortcuts whose variant name is derived:

```rust
Name |Type| { ... }  defines Type and emits Name(Type)
|Type| { ... }       defines Type and emits TypeParent(Type)
(Type)               uses Type and becomes TypeParent(Type)
```

To have multi-field variants carry one product instead, turn on
`product_variants`: `Pair(X, Y)` then emits `Pair((X, Y))`, and
`Span { start: usize }` emits `Span(ValueSpan)` with a generated `ValueSpan`:

```rust
strutuct! {
    #[strutuct(product_variants)]
    enum Value {
        Pair(String, String),
        Span { start: usize, end: usize },
        #[strutuct(no_product_variants)]
        Plain(String, String),
    }
}
```

The same attribute before a variant overrides the declaration-wide setting.

Configuration uses bare flags: `inclusions`, `product_variants`, and
`reverse_concat` enable options; their `no_` forms disable inherited options.
All three are disabled by default. Documentation is preserved as supplied,
without placeholders. The additional `allow_missing_docs` flag (`undoc`) permits
undocumented declarations and members throughout a generated branch.

`#[strutuct(reverse_concat)]` reverses automatically generated names,
turning `ParentName` into `NameParent`. The current complete option set is a
visibility, `inclusions`, `product_variants`, and `reverse_concat`. Options are
outer attributes, so they go where Rust accepts one: before the root
declaration, a named or tuple field, or a variant. They apply to that object
and everything generated below it. A type position, such as after `field:`,
is not an attribute position and rejects them.

The two visibility positions differ exactly in that reach. Written before a
declaration or field, a visibility belongs to that one object, as in Rust.
Written as an option, it becomes the default for that object and every
declaration and field generated below it, down to the leaves:

```rust
mod api {
    # use these_macros_should_be_illegal::strutuct;
    strutuct! {
        #[strutuct(pub(crate))]
        struct Request {
            method: Method { Get, Post },
            priv secret: Secret { token: String },
        }
    }
}
```

`Request`, `RequestMethod`, `RequestSecret` and their fields all become
`pub(crate)`, except the `secret` field itself: a written visibility, here
`priv`, wins locally and leaves the branch below it alone. `priv` is the
explicit private counterpart used to override an inherited visibility, and
`pub`, `pub(crate)`, `pub(super)` and `pub(in path)` are inherited as written.

As in Rust, attributes and visibility before a tuple element belong to that
field, and a variant takes no visibility: write a payload's visibility before
the payload type, as in `Branch(priv Branch { A, B })`.

When extended syntax could be read as unusually shaped Rust, `strutuct!` parses
it according to its own grammar. The spellings are chosen to keep realistic
collisions rare; ambiguous input is nevertheless resolved deterministically in
favor of the macro syntax.

Ordinary field and variant attributes are preserved locally. Root declaration
attributes are copied onto generated nested declarations, except documentation:
doc comments stay only where they were written. Derives are merged and
deduplicated; `#[underive(Trait)]` subtracts traits for one generated branch and
propagates that reduced list to its descendants. Other attributes, including
third-party attributes, are copied verbatim and remain responsible for whether
they accept each generated item shape. Generated enums use Rust's ordinary
`#[default]` variant attribute to select their default value. Generated
declarations suppress `dead_code`, constructor macros suppress `unused_macros`,
and `cfg`/`cfg_attr` guard both corresponding items.

Declarations appear directly as a field or tuple element type, optionally
wrapped by `?` or `*`. Any other Rust type, including everything inside generic
arguments, is copied unchanged, so `Box<dyn Fn(u8) -> u8>` keeps its meaning.

Each generated type also gets a constructor macro in Rust's conveniently
separate macro namespace. Every enum macro eats one path segment and calls the
next one:

```rust
A!(B::C::D(value))
```

folds into:

```rust
A::B(AB::C(ABC::D(value)))
```

A nested struct ends the path and accepts named fields. Boxed and optional edges
also stop there, so `T*` can express recursion without making the constructor
macro expand forever. That would be a different kind of illegal.

With `#[strutuct(inclusions)]`, enum trees additionally generate consuming
free functions for every constructor occurrence. For example,
`TokenOperatorNot(String) -> Token` composes the `Not` and `Operator`
injections. Complete occurrence paths keep equal leaf types in different
branches distinct. Joining stops at products, generic containers, and postfix
wrappers. A depth-`N` chain produces `N` functions whose total composed body
size is `O(N²)`—paths are never refactored or combinatorially enumerated.

## Transform macro structure itself

`reflect!` exchanges two macro invocation nodes around an opaque body:

```rust
let reflected = reflect!(add_one, double; 3);
// double! { add_one! { 3 } }
```

`perm!` applies conventional cycle notation to comma-separated token trees and
fixes every position beyond the largest mentioned index:

```rust
perm! { ((1 4 3)), a, b, c, d, e }
    -> c, b, d, a, e
```

See the [meta-transformer documentation](docs/meta-transformers.md) for the
expansion contract, right-to-left cycle composition, and raw-output caveat.

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

Or write the same option first in a whole-stream macro's own input:

```rust
literally_literal_string! {
    exclude_macros = (raw_tokens);

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

## License

Licensed under either the [MIT license](LICENSE) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option.
