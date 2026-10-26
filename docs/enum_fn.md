# `enum_fn`

API reference on docs.rs: [`enum_fn`](https://docs.rs/these-macros-should-be-illegal/latest/these_macros_should_be_illegal/attr.enum_fn.html).

Generates a method that matches enum variants to constants or values computed
from their borrowed fields.

## Arguments

```rust,ignore
#[enum_fn(const name -> Type, missing = expression)]
```

| Argument | Accepted value | Default |
| --- | --- | --- |
| `name` | Generated method's identifier | Required |
| `Type` | Exact Rust return type after `->` | Required |
| `missing` | Rust expression used by the wildcard arm | `::core::panic!()` |

Methods are runtime by default. Add `const` after the visibility to request a
`const fn`, for example `#[enum_fn(pub(crate) const code -> u8)]`. A trailing
comma is accepted.

Attributes before the signature inside the macro arguments apply to the
generated method. Add method documentation there when wanted:

```rust,ignore
#[enum_fn(#[doc = "Returns the protocol code."] pub const code -> u8)]
```

Without a method doc attribute, the generated method receives
`#[allow(missing_docs)]`. Attributes written outside `enum_fn` remain on the
enum.

The generated method always borrows `&self`. Return types and values are used
as written: there is no implicit `Option`, `Some`, conversion, or cloning.

## Fixed results

A non-lambda RHS supplies a fixed result, independent of the variant's fields.
It may be any Rust expression, including a function call; no lambda is needed
unless the result depends on payload fields.

```rust
# use these_macros_should_be_illegal::enum_fn;
#[enum_fn(const description -> u8)]
enum MixedDescription {
    Tuple = 1,
    Struct = 2,
    Dynamic(String) = 0,
}

const VALUE: &MixedDescription = &MixedDescription::Dynamic(String::new());
const CODE: u8 = VALUE.description();
assert_eq!(CODE, 0);
assert_eq!(MixedDescription::Tuple.description(), 1);
assert_eq!(MixedDescription::Struct.description(), 2);
```

Methods are runtime by default. Add `const` when the accessor should be usable
in a constant context. Rust then checks that all variant expressions and the
fallback are const-compatible. Const methods cannot use lambdas. Use a
zero-argument lambda for runtime computation on a unit variant.

Fixed-result methods preserve an elided return type such as `&str`; their
generated signatures rely on Rust's receiver lifetime elision. Lambda methods
use an explicit generated borrow lifetime so a result can borrow from fields.

## Lambdas

Each lambda must take exactly one argument per field. Arguments receive shared
references to the fields, and Rust checks their types and the returned value.
Borrowed results can live as long as the borrow of `self`.

For named fields, lambda parameters must use the field names in declaration
order. Tuple parameters may choose their own names. A unit variant, empty
struct variant, or empty tuple variant takes a zero-argument lambda.

```rust
# use these_macros_should_be_illegal::enum_fn;
#[enum_fn(code -> u8)]
enum Value {
    Named { code: u8 } = |code| *code,
    Pair(u8, u8) = |x, y| *x + *y,
    Empty {} = || 3,
    Unit = || 4,
}

assert_eq!(Value::Named { code: 7 }.code(), 7);
assert_eq!(Value::Pair(2, 3).code(), 5);
assert_eq!(Value::Empty {}.code(), 3);
assert_eq!(Value::Unit.code(), 4);
```

`Named { code: u8 } = |c| *c` is rejected because `c` renames the field.
`Pair(u8, u8) = |x| x.0 + x.1` is rejected because two fields require two
arguments. A single tuple-typed field remains one field:
`Pair((u8, u8)) = |pair| pair.0 + pair.1` is valid.

Lambdas make the method runtime-callable. A `const` declaration with a lambda
is rejected with a direct diagnostic.

## Missing values and non-exhaustive enums

A variant without an RHS reaches the wildcard arm. By default that arm calls
`panic!()`. Set `missing` to any expression of the declared return type:

```rust
# use these_macros_should_be_illegal::enum_fn;
#[enum_fn(const code -> u8, missing = 255)]
enum Status {
    Ready = 1,
    Unknown,
}

const UNKNOWN: u8 = Status::Unknown.code();
assert_eq!(Status::Ready.code(), 1);
assert_eq!(UNKNOWN, 255);
```

A custom panic is just an expression:
`#[enum_fn(code -> u8, missing = panic!("missing code"))]`.

The fallback also supports owned values and other runtime work:

```rust
# use these_macros_should_be_illegal::enum_fn;
#[enum_fn(message -> String, missing = String::from("not available"))]
enum Reply {
    Accepted = || String::from("accepted"),
    Other,
}

assert_eq!(Reply::Accepted.message(), "accepted");
assert_eq!(Reply::Other.message(), "not available");
```

The wildcard is always emitted, so `#[non_exhaustive]` works in either
attribute order. It is retained even when all declared variants have values;
its unreachable-pattern warning is suppressed locally. No return-type change
is inferred from exhaustiveness or missing values.

## Optional results

Write `Option<T>` in the signature, `Some(...)` on present values, and
`missing = None`. Lambdas must also explicitly return `Some(...)`:

```rust
# use these_macros_should_be_illegal::enum_fn;
#[enum_fn(description -> Option<&str>, missing = None)]
#[non_exhaustive]
enum Message {
    Fixed = Some("fixed text"),
    Owned(String) = |text| Some(text.as_str()),
    Missing,
}

assert_eq!(Message::Fixed.description(), Some("fixed text"));
assert_eq!(Message::Owned("owned".into()).description(), Some("owned"));
assert_eq!(Message::Missing.description(), None);
```

## Generics and conditional compilation

Enum attributes, visibility, generics, bounds, lifetimes, where clauses, and
field types are retained. Conditional compilation is forwarded to generated
implementations and arms. Nested `cfg_attr` is filtered so declaration-only
attributes such as `non_exhaustive` stay on declarations.

```rust
# use these_macros_should_be_illegal::enum_fn;
#[enum_fn(size -> usize)]
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

fn increment(value: u8) -> u8 { value + 1 }
let text = String::from("hello");
assert_eq!(Value::<String, 3>::Text(&text).size(), 5);
assert_eq!(Value::<String, 3>::Bytes([1, 2, 3]).size(), 3);
assert_eq!(Value::<String, 3>::Function(increment).size(), 3);
```
