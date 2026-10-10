# Changelog

## 0.9.0 - 2026-10-10

### Changed

- The `stringify_*_case!`, `stringify_lowercase!`, `stringify_uppercase!` and
  `stringify_type!` macros are replaced by one `stringify_as!`, whose leading
  option selects the spelling: `stringify_as!(snake; SomeName)`,
  `stringify_as!(type; Vec<T>)`.
- Attribute arguments accept a trailing comma uniformly, as in
  `#[callable(apply,)]`, `#[discriminated_str(name,)]` and `#[enum_fn(m: u8,)]`.
- `#[discriminated_str]` rejects attributes written on a discriminant literal,
  which previously had no effect; put them on the variant instead.
- `#[overload_op]` rejects negative and `default` impls, whose modifiers were
  previously ignored.
- Built on `syn` 3.

### Fixed

- Values forwarded by a `macro_rules!` caller as `$value:expr`, `$value:literal`
  or `$value:ty` are recognized like values written directly: string
  discriminants of `#[discriminated_str]`, field selectors and closures of
  `#[enum_fn]`, operand types of `#[overload_op]`, and `stringify_as!(type; ...)`,
  which previously spelled such a type as `$none(...)`.
- Clearer errors for unknown, repeated or list-form `exclude_macros` options,
  and for macro calls where a name or literal is required, since those
  arguments are read before any macro inside them expands.

## 0.8.0 - 2026-10-08

### Added

- `#[overload_op]` takes an operator implemented for references and generates
  the owned-operand forms and both augmented assignments, delegating to the
  borrowed impl without cloning. Works with any operator trait following the
  `Trait`/`TraitAssign` naming convention, written by name or by path.
- `#[complete_ops(Sub, Neg = <scalar>)]` completes subtraction from addition and
  negation, and negation from a scalar multiplication.
- `reflect!` accepts outer attributes before either invocation path and moves
  them together with that invocation.
- Book chapters on operator overloading, compile-time stringification, and
  `forward_attributes`, with a `strutuct!` grammar reference. Every book page
  with Rust examples is now compiled as a doctest.

### Changed

- `strutuct!` accepts a `struct` or `enum` keyword before an exact `|Payload|`
  enum member and validates it against the inferred shape.

### Fixed

- `strutuct!` rejects a visibility on an enum member that generates no payload
  type, instead of silently discarding it.
- `stringify_lowercase` and `stringify_uppercase` documentation now states that
  existing separators are retained.

## 0.7.0 - 2026-09-01

### Added

- `#[enum_fn(method: Type)]` generates a const or runtime method from arbitrary
  per-variant expressions, tuple and named field projections, or closures over
  the complete variant product. Missing arms can return `None`, stringify the
  variant name, or panic.
- `#[callable(method)]` and `make_fn!` provide same-name function-like syntax
  for a local value without erasing its nominal type or inherent methods.
- `stringify_*` macros convert identifiers and literals among common name cases,
  while `stringify_type!` produces a stable structure-preserving type name.
- `reflect!` reverses two nested macro invocation objects, and `perm!` applies
  right-to-left cycle products to comma-separated token trees.
- Runnable `enum_fn` and meta-transformer examples, expansion snapshots, and
  focused algebra, parser, generic-enum, attribute, and diagnostic tests.

### Changed

- `discriminated_str` now requires one unique string literal on every variant.
  It generates a `const fn -> &'static str` accessor and a same-name macro that
  selects the variant constructor from the literal.
- Macro implementations are organized by behavior under `flust`, `local`,
  `meta`, and shared `helpers`, while the crate root remains thin proc-macro
  bridges.

### Fixed

- Finite permutations now use one canonical source-to-destination map for cycle
  construction, multiplication, implicit embeddings, and token movement.
- `enum_fn` reports an out-of-range tuple projection at the attribute input
  instead of emitting an invalid generated match pattern.

### Migration

The former generalized `discriminated_str` accessor syntax has moved to
`enum_fn` and now requires an explicit return type:

```rust,ignore
// 0.6
#[discriminated_str(description)]

// 0.7
#[enum_fn(description: &str)]
```

Use the redesigned `#[discriminated_str(name)]` only when every variant has a
unique literal discriminant and the reverse literal-selected constructor macro
is useful.

## 0.6.0 - 2026-08-29

### Added

- `#[strutuct(inclusions = true)]` generates consuming functions for direct and
  iteratively joined enum constructor paths.
- `strutuct!` lowering now retains a reusable algebraic tree with a bottom-up
  fold, product selectors, coproduct branches, wrappers, and opaque Rust
  containers for future generated operations.

## 0.5.0 - 2026-08-29

### Changed

- Nested `strutuct!` declarations now derive their names from the complete
  generated parent path. For example, `Request { method: Method { ... } }` now
  generates `RequestMethod` instead of `Method`.
- Documentation attributes stay on the declaration, field, or variant where
  they were written instead of being copied through the generated subtree.
- Generated declarations suppress `dead_code`, while explicit user lint
  attributes can override that default.
- Conditional-compilation attributes now guard generated constructor macros as
  well as their corresponding types.

### Added

- Use `|Type|` to give an inline generated declaration an exact name instead of
  concatenating it with its parent.
- Use `#[underive(Trait)]` to remove inherited derives from one generated branch
  and its descendants.
- Nested declarations work throughout ordinary generic, tuple, array, grouped,
  optional, and boxed type positions.

### Fixed

- Derives and other inherited declaration attributes are no longer retargeted
  onto struct-like enum variants when `product_variants = false`.
- Internal placeholders used while hoisting declarations cannot collide with
  identifiers from the macro input.

### Migration

Code that referred to an automatically generated nested type by its former bare
name must use the new parent-qualified name:

```rust,ignore
// 0.4
let method = Method::Get;

// 0.5
let method = RequestMethod::Get;
```

When the old exact name is intentional, request it explicitly:

```rust,ignore
strutuct! {
    Request {
        method: |Method| { Get, Post },
    }
}
```
