# Changelog

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
