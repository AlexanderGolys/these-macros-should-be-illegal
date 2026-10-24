# `Log`

API reference on docs.rs: [`Log`](https://docs.rs/these-macros-should-be-illegal/latest/these_macros_should_be_illegal/derive.Log.html).

A struct is a product over its field names. Writing `E` for the set of those
names,

```rust,ignore
S  =  ∏ₑ∈ᴇ Xₑ
```

and when every field holds the same type `X` the product collapses to an
exponential:

```rust,ignore
S  =  X^E  =  Hom(E, X)
```

which makes `E` the logarithm of the struct to base `X` — hence the name. `Log`
generates that index set as an enum of unit variants, one per field, spelled
exactly as the field names. It only adds items beside the struct it reads, and
never rewrites it, which is what makes it a derive.

```rust,ignore,pane="You write"
#[derive(Log)]
struct BaseGroups {
    Keyword: Style,
    Function: Style,
    Bg: Style,
    Text: Style,
}
```

```rust,ignore,pane="Adds"
enum LogBaseGroups {
    Keyword,
    Function,
    Bg,
    Text,
}

impl Index<LogBaseGroups> for BaseGroups {
    type Output = Style;
    // one arm per field
}

impl IndexMut<LogBaseGroups> for BaseGroups { /* ... */ }
```

Field names are kept as they are written, raw-identifier escape included, so the
enum is the set of field names and nothing else. The enum allows
`non_camel_case_types`, since a field spelled to read as a variant is the whole
point. The struct stays the struct's own business, so spelling its fields that
way is worth an `#[allow(non_snake_case)]` of its own.

## The evaluation map is indexing

An exponential object comes with its evaluation map, `E × S → X`, which takes a
field name and a value and returns that field. Rust already spells that
signature in one trait, so the evaluation map is derived as
[`Index`](https://doc.rust-lang.org/core/ops/trait.Index.html), together with
its mutable half:

```rust
# #[macro_use] extern crate these_macros_should_be_illegal;
# use these_macros_should_be_illegal::Log;
# #[derive(Clone, Copy, Debug, PartialEq)]
# pub struct Style(u8);
#[derive(Log)]
#[allow(non_snake_case)]
pub struct BaseGroups {
    /// Style of a keyword.
    pub Keyword: Style,
    /// Style of the background.
    pub Bg: Style,
}

let mut groups = BaseGroups { Keyword: Style(1), Bg: Style(2) };
groups[LogBaseGroups::Bg] = Style(3);

assert_eq!(groups[LogBaseGroups::Keyword], Style(1));
```

Since the field names are ordinary values, the isomorphism also runs the other
way: the struct becomes a map out of its index set.

```rust
# #[macro_use] extern crate these_macros_should_be_illegal;
# use std::collections::HashMap;
# use these_macros_should_be_illegal::Log;
# #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
# pub struct Style(u8);
# #[derive(Log)]
# #[allow(non_snake_case)]
# pub struct BaseGroups {
#     /// Style of a keyword.
#     pub Keyword: Style,
#     /// Style of the background.
#     pub Bg: Style,
# }
let groups = BaseGroups { Keyword: Style(1), Bg: Style(2) };
let evaluated: HashMap<LogBaseGroups, Style> = [LogBaseGroups::Keyword, LogBaseGroups::Bg]
    .into_iter()
    .map(|name| (name, groups[name]))
    .collect();

assert_eq!(evaluated[&LogBaseGroups::Bg], Style(2));
```

That is what the default derives are for: without a written `derive`, the enum
derives `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash` and `Debug`, which an index
is expected to have.

## Naming the enum

The enum is named `Log` followed by the struct's name, and takes the struct's
visibility. The `log` helper attribute is the heading of its declaration, and
names it otherwise — the `enum` keyword is optional, a trailing comma is
accepted, and attributes written there replace the defaults:

```rust
# #[macro_use] extern crate these_macros_should_be_illegal;
# use these_macros_should_be_illegal::Log;
/// Distances along the axes of the plane.
#[derive(Log)]
#[log(#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Axis)]
pub struct Distances {
    /// Distance along the first axis.
    pub horizontal: u32,
    /// Distance along the second axis.
    pub vertical: u32,
}

let distances = Distances { horizontal: 3, vertical: 4 };
assert_eq!(distances[Axis::horizontal] + distances[Axis::vertical], 7);
```

A variant is documented where the field it names is not, and conditional
compilation written on a field is repeated on its variant and on both of its
arms, so a field compiled out takes its name with it.

## When no operator is derived

The index set is generated whatever the fields hold, but the evaluation map
needs one type to evaluate to:

- fields holding different types are a genuine product, `∏ₑ Xₑ`, with no `X` to
  index out of it, so the enum is generated alone;
- a struct with no fields generates the uninhabited enum, which is right: the
  empty product is a single point, and there is no field to evaluate.

## What is read, and when

The fields are read as the struct declares them. A tuple struct is therefore
rejected: its positions are already indexed by position, and there are no names
to generate. Attribute macros on the struct expand before any derive runs, which
is how [`forall`](forall.md) writes the fields this one reads — one type, one
visibility, and no repetition:

```rust
# #[macro_use] extern crate these_macros_should_be_illegal;
# use these_macros_should_be_illegal::{Log, forall};
# #[derive(Clone, Copy, Debug, PartialEq)]
# pub struct Style(u8);
/// Styles of the groups a theme is written in.
#[forall(#[doc = "Style of one group."] pub, Style)]
#[derive(Log)]
#[log(pub Group)]
#[allow(non_snake_case)]
pub struct Theme(Keyword, Function, Bg);

let theme = Theme { Keyword: Style(1), Function: Style(2), Bg: Style(3) };

assert_eq!(theme[Group::Function], Style(2));
```
