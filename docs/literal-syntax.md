# Literal syntax and external modules

API reference on docs.rs: [`literally_literal_string!`](https://docs.rs/these-macros-should-be-illegal/latest/these_macros_should_be_illegal/macro.literally_literal_string.html), [`expand!`](https://docs.rs/these-macros-should-be-illegal/latest/these_macros_should_be_illegal/macro.expand.html).

## `literally_literal_string!`

`literally_literal_string!` turns `@@"text"` into an owned `String`:

```rust,pane="You write"
# #[macro_use] extern crate these_macros_should_be_illegal;
let greeting: String = literally_literal_string!(@@"hello");
assert_eq!(greeting, "hello");
```

```rust,ignore,pane="Roughly expands to"
let greeting: String = ::std::string::String::from("hello");
assert_eq!(greeting, "hello");
```

That spelling is not valid ordinary Rust. It works inside a function-like macro
because rustc gives the macro its input tokens before parsing them as an
expression.

## `expand!`

At item level, invalid syntax must live in a separate file so rustc does not
parse it first. `expand!` loads an out-of-line module and wraps its body in the
selected rewriting macros.

```rust,ignore,pane="You write"
expand!(
    literally_literal_string;
    mod experiments;
);

// experiments.rs
pub fn greeting() -> String {
    @@"hello from inadvisable Rust"
}
```

```rust,ignore,pane="Roughly expands to"
mod experiments {
    pub fn greeting() -> String {
        ::std::string::String::from(
            "hello from inadvisable Rust",
        )
    }
}
```

Multiple macro paths are applied in their written order. `expand!` currently
loads one module file; it does not recursively load out-of-line child modules.
Cargo may also miss a change made only to a file read by the procedural macro,
so force a rebuild if an edit appears to be ignored.

## Excluding macro inputs

Some macros own private token syntax that another transformation must not
touch. Configure exact macro names in `expand!`:

```rust,ignore,pane="You write"
expand!(
    literally_literal_string,
    exclude_macros = (raw_tokens);
    mod experiments;
);

// experiments.rs
raw_tokens!(@@"untouched");
let text = @@"rewritten";
```

```rust,ignore,pane="Roughly expands to"
mod experiments {
    raw_tokens!(@@"untouched");
    let text = ::std::string::String::from("rewritten");
}
```

A whole-stream macro invoked directly takes the same option as the first thing
in its input, ended by a semicolon:

```rust,ignore
literally_literal_string! {
    exclude_macros = (raw_tokens);

    raw_tokens!(@@"untouched");
    let text = @@"rewritten";
}
```
