# Changelog

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
