# `shared_match_arms!`

API reference on docs.rs: [`shared_match_arms!`](https://docs.rs/these-macros-should-be-illegal/latest/these_macros_should_be_illegal/macro.shared_match_arms.html).

Rust or-patterns require every repeated binding to have the same concrete type.
`shared_match_arms!` instead clones one RHS into several ordinary match arms,
so each binding is type-checked independently:

```rust,pane="You write"
# #[macro_use] extern crate these_macros_should_be_illegal;
enum Value {
    Number(u32),
    Character(char),
}

let value = Value::Number(3);
let text = shared_match_arms! {
    match value {
        Value::Number(value) || Value::Character(value) => value.to_string(),
    }
};

assert_eq!(text, "3");
```

```rust,ignore,pane="Roughly expands to"
enum Value {
    Number(u32),
    Character(char),
}

let value = Value::Number(3);
let text = match value {
    Value::Number(value) => value.to_string(),
    Value::Character(value) => value.to_string(),
};

assert_eq!(text, "3");
```

The invented `||` joins complete arm patterns, not Rust or-patterns. The
expansion above contains two independent arms with the same RHS.

## Guards

Parenthesize a component when it has its own guard:

```rust,pane="You write"
# #[macro_use] extern crate these_macros_should_be_illegal;
enum Value {
    Number(u32),
    Character(char),
}

let value = Value::Number(3);
let selected = shared_match_arms! {
    match value {
        (Value::Number(value) if value > 0)
            || (Value::Character(value) if value.is_ascii())
            => true,
        _ => false,
    }
};

assert!(selected);
```

```rust,ignore,pane="Roughly expands to"
enum Value {
    Number(u32),
    Character(char),
}

let value = Value::Number(3);
let selected = match value {
    Value::Number(value) if value > 0 => true,
    Value::Character(value) if value.is_ascii() => true,
    _ => false,
};

assert!(selected);
```

Logical OR remains ordinary Rust when it occurs inside a component guard:

```rust,ignore
(Pattern(value) if first(value) || second(value)) || Other(value) => rhs
```

## Whole-stream rewriting

Like `literally_literal_string!`, this macro accepts an arbitrary token stream
and searches recursively through its groups. Attributes stay opaque, and a
leading `exclude_macros = (names);` option protects macro inputs with private
syntax. It can also be listed in `expand!` when the extended syntax lives in an
out-of-line module.

This is deliberately a function-like macro. An attribute macro would require
rustc to accept the surrounding item before this expression-level extension
could reliably rewrite it.
