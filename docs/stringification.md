# Compile-time stringification

The stringification macros turn one name or Rust type into a string literal at
macro-expansion time. Their output is an ordinary `&'static str`, so it works in
constants, patterns, generated attributes, and other constant contexts without
runtime allocation.

## Name cases

Each name macro accepts exactly one non-empty identifier or string literal.
Keywords and raw identifiers are accepted as names; the raw `r#` marker is not
part of the result.

```rust
use these_macros_should_be_illegal::{
    stringify_camel_case, stringify_kebab_case, stringify_lowercase,
    stringify_pascal_case, stringify_screaming_snake_case,
    stringify_snake_case, stringify_uppercase,
};

const CAMEL: &str = stringify_camel_case!(some_HTTP_server);
const PASCAL: &str = stringify_pascal_case!(some_HTTP_server);
const SNAKE: &str = stringify_snake_case!(SomeHTTPServer);
const KEBAB: &str = stringify_kebab_case!(SomeHTTPServer);
const SHOUTING: &str = stringify_screaming_snake_case!(SomeHTTPServer);
const LOWER: &str = stringify_lowercase!(Some_HTTP_Server);
const UPPER: &str = stringify_uppercase!(Some_HTTP_Server);

assert_eq!(CAMEL, "someHttpServer");
assert_eq!(PASCAL, "SomeHttpServer");
assert_eq!(SNAKE, "some_http_server");
assert_eq!(KEBAB, "some-http-server");
assert_eq!(SHOUTING, "SOME_HTTP_SERVER");
assert_eq!(LOWER, "some_http_server");
assert_eq!(UPPER, "SOME_HTTP_SERVER");
```

The supported conversions are:

| Macro | Result style |
| --- | --- |
| `stringify_camel_case!` | `lowerCamelCase` |
| `stringify_pascal_case!` | `UpperCamelCase` or `PascalCase` |
| `stringify_snake_case!` | `snake_case` |
| `stringify_kebab_case!` | `kebab-case` |
| `stringify_screaming_snake_case!` | `SCREAMING_SNAKE_CASE` |
| `stringify_lowercase!` | Unicode lowercase; existing separators remain |
| `stringify_uppercase!` | Unicode uppercase; existing separators remain |

A string literal contributes its contents rather than its Rust source spelling,
so this also converts names that cannot be written as one identifier:

```rust
use these_macros_should_be_illegal::{
    stringify_pascal_case, stringify_uppercase,
};

assert_eq!(stringify_pascal_case!("some-http-server"), "SomeHttpServer");
assert_eq!(stringify_uppercase!(r#type), "TYPE");
```

The conventional word-based conversions use acronym and separator boundaries.
The plain lowercase and uppercase variants only change character case; they do
not otherwise normalize the separators.

## Rust types

`stringify_type!` parses exactly one Rust type and emits a compact spelling with
the prefix `type:`:

```rust
use these_macros_should_be_illegal::stringify_type;

const SIMPLE: &str = stringify_type!(String);
const BORROWED: &str = stringify_type!(&'static mut Vec<Option<String>>);
const FUNCTION: &str =
    stringify_type!(fn((u8, u16), *const [u8; 4]) -> bool);

assert_eq!(SIMPLE, "type:String");
assert_eq!(BORROWED, "type:&'static mut Vec<Option<String>>");
assert_eq!(FUNCTION, "type:fn((u8,u16),*const[u8;4])->bool");
```

Whitespace and source formatting disappear, while paths, lifetimes, pointer
kinds, delimiters, and other type punctuation remain. The deliberately illegal
identifier prefix keeps a type name such as `type:String` distinct from an
ordinary identifier-derived name.

This is canonicalization of written Rust type syntax, not compiler type
identity. It does not resolve aliases, imports, or whether two differently
written types happen to denote the same semantic type.
