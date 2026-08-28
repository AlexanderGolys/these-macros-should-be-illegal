# These Macros Should Be Illegal

A personal Rust procedural-macro laboratory for ideas that probably should not
be this easy to express. Idiomatic boundaries are optional here: whole-module
transformations and deliberately cursed generated code are welcome.

The root package is a procedural-macro crate for the collection, while
experiments that need separate dependencies or release cycles can live under
[`crates/`](crates/).

## Literally literal strings

`literally_literal_string!` is a plain function-like macro which transforms
only the token stream supplied to it:

```rust
use these_macros_should_be_illegal::literally_literal_string;

let greeting = literally_literal_string!(@@"hello from inadvisable Rust");
```

The shared `expand!` macro keeps an entire side-module's custom syntax out of
Rust's parser without wrapping or indenting that file. List the transformations
to inject, followed by the out-of-line module declaration:

```rust
// src/lib.rs
these_macros_should_be_illegal::expand!(
    these_macros_should_be_illegal::literally_literal_string;
    mod experiments;
);
```

```rust
// src/experiments.rs
pub fn greeting() -> String {
    @@"hello from inadvisable Rust"
}
```

`expand!` reads and lexes `experiments.rs`, then emits an inline module whose
body is wrapped in the listed function-like macros. Rust therefore parses only
their final output. The crate targets stable Rust. Cargo may not notice changes
made only to files read by `expand!`; rerun the build when necessary.

The first rewrite recursively treats two adjacent `@` punctuation tokens
followed by a Rust string-literal token as an owned string:

```text
@@"hello"  ->  ::std::string::String::from("hello")
```

It performs a token-tree transformation, not source-text substitution. Both
outer (`#[...]`) and inner (`#![...]`) attributes are copied as opaque tokens,
as are macro invocations explicitly named in `exclude_macros`. Declarative
macro definitions are traversed like every other group. The current prototype
loads one out-of-line module file; recursive external child-module loading
remains future work.

Shared configuration can be written directly in `expand!`:

```rust
these_macros_should_be_illegal::expand!(
    these_macros_should_be_illegal::literally_literal_string,
    exclude_macros = (raw_tokens);
    mod experiments;
);
```

For any configuration-aware, item-position function-like macro in this crate,
the generic exclusion wrapper provides the same option without changing that
macro's public grammar:

```rust
#[these_macros_should_be_illegal::excluded_macros(raw_tokens)]
literally_literal_string! {
    // transformed tokens
}
```

## String discriminants

`#[str_disc(method)]` keeps an enum's string metadata beside each variant and
generates a borrowed accessor:

```rust
use these_macros_should_be_illegal::str_disc;

#[str_disc(description)]
pub enum Action {
    Quit = "quit the application",
    Submit = "evaluate the input",
}
```

This becomes a unit-variant enum with
`pub const fn description(&self) -> &'static str`. Select an allocating
`String` or lifetime-elided `&str` accessor explicitly when desired:

```rust
#[str_disc(description: String)]
enum OwnedDescription { Example = "allocated on access" }

#[str_disc(description: &str)]
enum BorrowedDescription { Example = "borrowed" }
```

Those typed forms generate ordinary, non-`const` methods.

Variants may retain ordinary tuple or struct payloads alongside a fixed
description. A variant with no discriminant must contain exactly one `String`;
its value becomes the description:

```rust
#[str_disc(description)]
enum Error {
    Io(std::io::Error) = "I/O error",
    Parse { line: usize } = "parse error",
    Custom(String),
}
```

When an enum contains a dynamic description, the default accessor is an
ordinary `fn description(&self) -> &str`. An all-fixed enum retains its
`const fn -> &'static str` accessor.

## Nested algebraic declarations

`strutuct!` generates one struct or enum together with declarations nested in
its fields or variants. A body beginning with `name: Type` is a struct; any
variant-shaped body is an enum:

```rust
use these_macros_should_be_illegal::strutuct;

strutuct! {
    S
    a: A { A1, A2, A3 },
    b: B,
}
```

This emits public `A` and `S` declarations, with `A` ordered before the struct
that uses it. Parenthesized types create automatically named variants, while
ordinary named constructors remain unchanged:

```rust
strutuct! {
    Expr
    Unary { (Pref), (Post) }
    (Bin)
    LitStr(String)
    Null
}
```

The generated enums contain `UnaryPref(Pref)`, `UnaryPost(Post)`,
`ExprUnary(Unary)`, `ExprBin(Bin)`, `LitStr(String)`, and `Null`. Each generated
type also receives a constructor macro in Rust's separate macro namespace:

```rust
let prefix = Expr!(Unary::Pref(1));
let literal = Expr!(LitStr("text".to_owned()));
let value = S!(a: A!(A2), b: B(7));
```

Each enum macro consumes one nested path segment and invokes the next enum's
macro, so `A!(B::C::D::Variant(value))` folds to
`A::AB(B::BC(C::CD(D::Variant(value))))`. A nested struct ends the path and
accepts its fields with `A!(B { field: value })`; a root struct accepts the same
field body directly.

Postfix `T?` and `T*` lower to `Option<T>` and `Box<T>`, respectively. Wrapped
edges are terminal for recursive constructor macros, allowing `*` to mark
recursive indirection without producing an infinite expansion.

## Adding a macro

Add closely related macros to the root crate. For an independent experiment,
create a library crate from the repository root:

```sh
cargo new --lib crates/example-macro
```

Then mark it as a procedural macro in its `Cargo.toml`:

```toml
[lib]
proc-macro = true
```

Workspace crates should inherit the shared edition and lint configuration where
appropriate. Add unit tests for parsing and generation, integration tests for
consumer behavior, and compile-fail tests for diagnostics.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace --all-features
```

## Publishing and license

Releases are published through GitHub and crates.io. The project is available
under the [MIT License](LICENSE).
