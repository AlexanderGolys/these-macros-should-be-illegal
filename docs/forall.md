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

The prefix is made of five kinds of part, each recognized by the syntax that
only it has, so they may be written in any order:

| Part | Written as | How many |
| --- | --- | --- |
| Outer attributes | `#[derive(Debug)]` | any number |
| Visibility | `pub`, `pub(crate)`, `pub(in path)` | one |
| Shared types | `T`, `Box<T>`, `[u8; 4]`, `fn(u8) -> u8` | one per field of a struct, any number for an enum |
| Shared named fields | `{ code: u16, path: String }` | one group, for an enum |
| Type constructor | `Option<>`, `Box<>` | any number, lifts the field types |

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

## Lifting the field types

A type written with empty generics, `F<>`, is not a type at all, so `forall`
reads it as a constructor and wraps the type of every declared field in it,
`T` becoming `F<T>`:

```rust,pane="You write"
# #[macro_use] extern crate these_macros_should_be_illegal;
#[forall(Option<>)]
struct Patch {
    name: String,
    tags: Vec<String>,
}
```

```rust,ignore,pane="Roughly expands to"
struct Patch {
    name: Option<String>,
    tags: Option<Vec<String>>,
}
```

Structs, unions and every kind of enum variant are lifted alike. The parts of
a prefix are applied one after another in the order they are written, as if each
were its own `#[forall(...)]` stacked on the item, since an attribute macro runs
before the ones below it. A constructor therefore lifts what is already there:
every declared field, and the shared types and fields written before it. What is
written after it is added later and stays as written:

```rust,pane="You write"
# #[macro_use] extern crate these_macros_should_be_illegal;
#[forall(u8, Option<>, u16)]
enum Sample {
    Reading(i8),
}
```

```rust,ignore,pane="Roughly expands to"
enum Sample {
    Reading(Option<i8>, Option<u8>, u16),
}
```

For a tuple struct whose fields are bare names, the shared type has to be
written before the constructor: `#[forall(u8, Option<>)] struct S(a)` goes
through `S { a: u8 }` to `S { a: Option<u8> }`. Written the other way round, the
constructor would have to lift the bare name `a`, which is rejected.

Several constructors nest with the last one outermost, so
`forall(Box<>, Option<>)` turns `T` into `Option<Box<T>>`. A member opts out of
one constructor with `#[tmsbi(skip(Option<>))]`.

### Why a prefix reads left to right

`forall(a, b)` is sugar for stacking the parts as separate attributes, in the
order they are written:

```rust,ignore
#[forall(a, b)]    ≡    #[forall(a)]
                        #[forall(b)]
```

That is not a free choice, and it is worth seeing why nothing else is possible.
An attribute macro does not receive the attributes below it as already
expanded code. It receives the item with those attributes still attached, runs
first, and only then does the compiler go on to the next one. The top attribute
therefore always acts first, on the item as the user wrote it.

Now let `f` be any attribute that inserts a `forall` of its own, say
`#[f]` ⇒ `#[forall(A<>)]`, and write

```rust,ignore,pane="You write"
#[forall(B<>)]
#[f]
struct S(T);
```

```rust,ignore,pane="Step by step"
#[forall(B<>)] #[f] struct S(T);
      ↦ #[f] struct S(B<T>);
      ↦ #[forall(A<>)] struct S(B<T>);
      ↦ struct S(A<B<T>>);
```

Could `forall(B<>)` do anything to come out as `B<A<T>>` instead? It would have
to know that `f` is about to wrap the field and make room for it, which means
expanding `f` itself, and `f` can be any macro anybody has ever written.
`forall` cannot recognize it, so to `forall` it is indistinguishable from
`#[derive(Debug)]` or `#[allow(dead_code)]`, and the only thing it can do with
the attributes below is leave them alone. Whatever order the lifting has, it is
the order of the stack, top to bottom, and a prefix, being sugar for a stack,
has the same one.

The shape of this is worth a name. Writing the stack as function application,
`forall(a)(forall(b)(X))`, puts `b` innermost, yet it is `a` that runs first.
The textual nesting and the order of execution are dual to each other, as
usual for macros: the outermost macro is the first to see the item. No amount
of care inside `forall` turns one into the other, any more than waving hands
turns a presheaf into a copresheaf. Hence the rule, in the small:

- the part written first lifts first, so it ends up innermost;
- a constructor lifts what is already there, namely the declared fields and
  the shared types and fields written before it;
- what is written after it is added later, and stays as written.

### Attributes in a stack and in a prefix

Attributes are distributed by putting them in front of the member's own. In a
stack, each `forall` does so in turn, so the one that runs last ends up at the
very top. With `#[forall(#[a])] #[forall(#[b])]` the member carries `#[b]`
and then `#[a]`.

Types and attributes therefore agree across a stack: the `forall` that runs
last is outermost for a type and topmost for attributes, and the one that runs
first sits closest to the item.

Inside one prefix the attributes are the exception. They keep the order they
are written in, `forall(#[a], #[b])` giving `#[a]` and then `#[b]`, because
reversing a list of attributes someone has just written would be the surprising
choice. The attributes of a prefix are thus not the stack of its parts, whereas
its types are. If you want the stacked order for attributes, write separate
`forall`s; if the order of some attributes matters, put them in one prefix, in
the order they should appear.

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
