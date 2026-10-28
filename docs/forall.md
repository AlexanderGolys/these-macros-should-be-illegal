# `forall`

API reference on docs.rs: [`forall`](https://docs.rs/these-macros-should-be-illegal/latest/these_macros_should_be_illegal/attr.forall.html).

`forall` writes the constant part of a member declaration once. Its arguments
read like the beginning of one field, `#[attribute] pub Type`, and are repeated
on every member of the item below: on each field of a struct or union, and on
each variant of an enum.

```rust,pane="You write"
# #[macro_use] extern crate these_macros_should_be_illegal;
#[forall(pub(crate))]
pub struct Pixel(u8, u8, u8);
```

```rust,ignore,pane="Roughly expands to"
pub struct Pixel(
    pub(crate) u8,
    pub(crate) u8,
    pub(crate) u8,
);
```

## The prefix

The prefix is made of four kinds of part, each recognized by the syntax that
only it has, so they may be written in any order:

| Part | Written as | How many |
| --- | --- | --- |
| Outer attributes | `#[derive(Debug)]` | any number |
| Visibility | `pub`, `pub(crate)`, `pub(in path)` | one |
| Shared types | `T`, `Box<T>`, `[u8; 4]`, `fn(u8) -> u8` | one per field of a struct, any number for an enum |
| Shared named fields | `{ code: u16, path: String }` | one group, for an enum |

Commas between the parts are optional, exactly as in the declaration the prefix
is written like. A member keeps what it declares itself: its own visibility
survives the distributed one, and the shared attributes are added in front of
the ones it already carries.

Only outer attributes are distributed. An inner attribute is rejected, since no
field or variant can carry one, and a group must name at least one field.

## What each item takes

A braced struct and a union take attributes and a visibility. Their fields
already name the type they hold, so a shared type is rejected rather than
silently dropped:

```rust
# #[macro_use] extern crate these_macros_should_be_illegal;
# use these_macros_should_be_illegal::forall;
#[forall(#[doc = "One shared field."] pub)]
struct Window {
    width: u32,
    pub(crate) height: u32,
}
```

A tuple struct takes one shared type as well, and then its fields are written
as bare names, which Rust itself has no syntax for:

```rust,pane="You write"
# #[macro_use] extern crate these_macros_should_be_illegal;
#[forall(pub, f32)]
struct Point(x, y);
```

```rust,ignore,pane="Roughly expands to"
struct Point {
    pub x: f32,
    pub y: f32,
}
```

An enum takes attributes and the fields of its variants, written either as the
shared types or as one group of shared named fields. Variants have no
visibility, so distributing one is rejected:

```rust,pane="You write"
# #[macro_use] extern crate these_macros_should_be_illegal;
#[forall(u16, String)]
enum Response {
    Redirect,
    Error(&'static str),
}
```

```rust,ignore,pane="Roughly expands to"
enum Response {
    Redirect(u16, String),
    Error(&'static str, u16, String),
}
```

<!-- -->

```rust,pane="You write"
# #[macro_use] extern crate these_macros_should_be_illegal;
#[forall({ code: u16 })]
enum Request {
    Read,
    Write { payload: u8 },
}
```

```rust,ignore,pane="Roughly expands to"
enum Request {
    Read { code: u16 },
    Write { payload: u8, code: u16 },
}
```

The shared fields are added after the ones a variant declares, so a variant
holding unnamed fields takes the shared types, and a variant naming its fields
takes the shared named ones. The other combination is rejected, as is a variant
with a discriminant, which cannot hold fields at all.

## Keeping a member out

A member that should not take the prefix carries `#[tmsbi(skip)]`. To keep
out only some of it, list those fragments, written exactly as in the prefix:

```rust,pane="You write"
# #[macro_use] extern crate these_macros_should_be_illegal;
#[forall(#[doc = "Shared state."] #[serde(default)] pub)]
#[derive(serde::Deserialize)]
struct Config {
    retries: u8,
    #[tmsbi(skip(#[serde]))]
    /// Must always be given.
    endpoint: String,
    #[tmsbi(skip)]
    cache: Vec<u8>,
}
```

```rust,ignore,pane="Roughly expands to"
struct Config {
    /// Shared state.
    #[serde(default)]
    pub retries: u8,
    /// Shared state.
    /// Must always be given.
    pub endpoint: String,
    cache: Vec<u8>,
}
```

An attribute written as a bare path, like `#[serde]` above, skips every shared
attribute of that path, while one written in full skips only its identical
twin. On an enum the shared fields are skipped the same way, which leaves room
for a variant that holds none of them:

```rust,pane="You write"
# #[macro_use] extern crate these_macros_should_be_illegal;
#[forall(u16, String)]
enum Reply {
    Moved,
    #[tmsbi(skip(String))]
    Empty,
    #[tmsbi(skip)]
    Unknown,
}
```

```rust,ignore,pane="Roughly expands to"
enum Reply {
    Moved(u16, String),
    Empty(u16),
    Unknown,
}
```

The marker is inert: every `forall` stacked on the item reads it, and the last
one removes it. One marker therefore speaks to all of them, and each prefix
skips only the fragments it distributes:

```rust
# #[macro_use] extern crate these_macros_should_be_illegal;
#[forall(pub)]
#[forall(u8)]
struct Range(start, #[tmsbi(skip(pub))] end);
```

A field named from its tuple position must still take the shared type, so it
can skip the other fragments but not the type itself.

The marker lives under `tmsbi`, the namespace of this crate, so it does not
collide with attributes of other crates. `forall` claims only the `tmsbi`
markers opening with `skip` and leaves the rest alone.

## Generated documentation

A named field that `forall` creates is documented where the declaration
documents none, so the macro stays usable under `#![deny(missing_docs)]`.
Documentation written on a tuple position is kept, and attributes written there
survive the renaming:

```rust
# #[macro_use] extern crate these_macros_should_be_illegal;
# use these_macros_should_be_illegal::forall;
#[forall(pub, u8)]
struct Channels(
    #[doc = "Intensity of the red channel."] red,
    green,
);
```

`green` is documented as a field generated by `forall`; `red` keeps the
documentation written for it.

## Forwarded prefixes

A prefix forwarded by a declarative macro is read like one written directly,
through the invisible groups that `macro_rules!` puts around its fragments:

```rust
# #[macro_use] extern crate these_macros_should_be_illegal;
# use these_macros_should_be_illegal::forall;
macro_rules! shared_fields {
    ($name:ident : $visibility:vis $shared:ty { $($field:ident),* }) => {
        /// Struct generated with one shared field type.
        #[forall($visibility $shared)]
        pub struct $name($($field),*);
    };
}

shared_fields!(Forwarded: pub u16 { first, second });
```

The visibility may be a `vis` fragment that matched nothing, which distributes
no visibility at all, and a field name may itself arrive as a `ty` fragment.

## Diagnostics

Every rejection names the member it is about:

- a shared type for fields that already name theirs, or more than one shared
  type for a struct, whose fields hold exactly one;
- a visibility for the variants of an enum;
- a group of shared named fields anywhere but an enum;
- both shared types and a group of named fields in one prefix;
- a prefix for an item without a single field or variant to repeat it on;
- a type where a bare field name is expected, which is what a tuple position
  means once the prefix names the shared type;
- a `#[tmsbi(skip(...))]` marker followed by anything after its fragments, or
  one skipping the shared type of a field named from its tuple position;
- an inner attribute, a second visibility, a second group of named fields, or a
  group naming no field at all.
