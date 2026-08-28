# Literal syntax and external modules

## `literally_literal_string!`

`literally_literal_string!` turns `@@"text"` into an owned `String`:

```rust
use these_macros_should_be_illegal::literally_literal_string;

let greeting: String = literally_literal_string!(@@"hello");
assert_eq!(greeting, "hello");
```

That spelling is not valid ordinary Rust. It works inside a function-like macro
because rustc gives the macro its input tokens before parsing them as an
expression.

## `expand!`

At item level, invalid syntax must live in a separate file so rustc does not
parse it first. `expand!` loads an out-of-line module and wraps its body in the
selected rewriting macros.

```rust
use these_macros_should_be_illegal::expand;

expand!(
    literally_literal_string;
    mod experiments;
);
```

The corresponding `experiments.rs` may contain:

```rust,ignore
pub fn greeting() -> String {
    @@"hello from inadvisable Rust"
}
```

Multiple macro paths are applied in their written order. `expand!` currently
loads one module file; it does not recursively load out-of-line child modules.
Cargo may also miss a change made only to a file read by the procedural macro,
so force a rebuild if an edit appears to be ignored.

## Excluding macro inputs

Some macros own private token syntax that another transformation must not
touch. Configure exact macro names in `expand!`:

```rust,ignore
expand!(
    literally_literal_string,
    exclude_macros = (raw_tokens);
    mod experiments;
);
```

The `excluded_macros` attribute provides the same configuration when wrapping
a configuration-aware item-position transformation.
