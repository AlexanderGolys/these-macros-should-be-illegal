# Compile-time stringification

API reference: [`stringify_as!`](https://docs.rs/these-macros-should-be-illegal/latest/these_macros_should_be_illegal/macro.stringify_as.html) on docs.rs.

`stringify_as!` turns one name or Rust type into a string literal at
macro-expansion time; the option before `;` selects the spelling. The output is
an ordinary `&'static str`, so it works in constants, patterns, generated
attributes, and other constant contexts without runtime allocation.

## Name cases

Each case option accepts exactly one non-empty identifier or string literal.
Keywords and raw identifiers are accepted as names; the raw `r#` marker is not
part of the result.

```rust
const CAMEL: &str = stringify_as!(camel; some_HTTP_server);
const PASCAL: &str = stringify_as!(pascal; some_HTTP_server);
const SNAKE: &str = stringify_as!(snake; SomeHTTPServer);
const KEBAB: &str = stringify_as!(kebab; SomeHTTPServer);
const SHOUTING: &str = stringify_as!(screaming_snake; SomeHTTPServer);
const LOWER: &str = stringify_as!(lower; Some_HTTP_Server);
const UPPER: &str = stringify_as!(upper; Some_HTTP_Server);

assert_eq!(CAMEL, "someHttpServer");
assert_eq!(PASCAL, "SomeHttpServer");
assert_eq!(SNAKE, "some_http_server");
assert_eq!(KEBAB, "some-http-server");
assert_eq!(SHOUTING, "SOME_HTTP_SERVER");
assert_eq!(LOWER, "some_http_server");
assert_eq!(UPPER, "SOME_HTTP_SERVER");
```

The supported conversions are:

| Option | Result style |
| --- | --- |
| `camel` | `lowerCamelCase` |
| `pascal` | `UpperCamelCase` or `PascalCase` |
| `snake` | `snake_case` |
| `kebab` | `kebab-case` |
| `screaming_snake` | `SCREAMING_SNAKE_CASE` |
| `lower` | Unicode lowercase; existing separators remain |
| `upper` | Unicode uppercase; existing separators remain |

A string literal contributes its contents rather than its Rust source spelling,
so this also converts names that cannot be written as one identifier:

```rust
assert_eq!(stringify_as!(pascal; "some-http-server"), "SomeHttpServer");
assert_eq!(stringify_as!(upper; r#type), "TYPE");
```

The conventional word-based conversions use acronym and separator boundaries.
The plain `lower` and `upper` options only change character case; they do
not otherwise normalize the separators.

## Rust types

The `type` option parses exactly one Rust type and emits a compact spelling with
the prefix `type:`:

```rust
const SIMPLE: &str = stringify_as!(type; String);
const BORROWED: &str = stringify_as!(type; &'static mut Vec<Option<String>>);
const FUNCTION: &str =
    stringify_as!(type; fn((u8, u16), *const [u8; 4]) -> bool);

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
