# Changelog

## 0.12.2 - 2026-10-28

### Changed

- The `#[forall]` member marker is spelled `#[tmsbi(skip)]` and
  `#[tmsbi(skip(fragments))]`, under the `tmsbi` namespace of this crate, in
  place of `#[forall(skip)]`. `forall` leaves other `tmsbi` markers to their
  own macros.

## 0.12.1 - 2026-10-28

### Added

- `#[forall]` members can keep out of the prefix with `#[forall(skip)]`, or
  keep out only some fragments with `#[forall(skip(fragments))]`. The marker is
  inert and is read by every stacked `forall`. The last one removes it.

## 0.12.0 - 2026-10-26

### Changed

- **Breaking:** `#[enum_fn]` declares its exact return type with `->`, uses
  explicit `missing = expression` behavior, and returns the declared type
  without inferring `Option` or wrapping values.
- **Breaking:** `#[enum_fn]` accessors are runtime methods by default. Add
  `const` after visibility when a const accessor is required; Rust checks all
  values and the fallback for const compatibility.
- `#[enum_fn]` accepts fixed expressions, including runtime function calls,
  and lambdas over borrowed variant fields. Method-level attributes, including
  `#[doc = "..."]`, can be written before the signature in the macro input.
- The generated accessor no longer receives synthesized documentation. When
  no method doc attribute is supplied, only that method gets
  `#[allow(missing_docs)]`.

### Migration

Replace field selectors and inferred optional results with lambdas and explicit
`Option` values. Add `const` to accessors used from constants. For example:

```rust,ignore
#[enum_fn(const description -> Option<&'static str>, missing = None)]
enum State {
    Ready = Some("ready"),
    Unknown,
}
```

## 0.11.0 - 2026-10-24

### Added

- `#[derive(Log)]` generates an enum of a struct's field names and, for
  homogeneous fields, `Index` and `IndexMut` implementations. The `#[log(...)]`
  helper configures the generated enum's name, visibility, and attributes.
- `#[macro_args(...)]` forwards separate parameters into a function-like macro's
  input as `@macro_args(...);`. Every function-like macro recognizes that
  envelope; macros without separate parameters accept only an empty list.
- Directly importable aliases: `lls!` for `literally_literal_string!`, `disctr`
  for `discriminated_str`, and `sma!` for `shared_match_arms!`.
- Short option names: `inc`, `prod`, `rev`, and `exclude`. Flag aliases also
  accept their `no_` forms and share duplicate validation with full names.
- `strutuct!` accepts `allow_missing_docs` (`undoc`) to allow undocumented
  declarations and members throughout a branch, including hoisted nested
  declarations. Its negated forms are `no_allow_missing_docs` and `no_undoc`.
- Explicit `struct` and `enum` declaration shapes, ordinary Rust visibility,
  branch-local configuration, inherited derives, and `#[underive(...)]`
  handling in the reworked `strutuct!` parser and lowering.

### Changed

- **Breaking:** `#[enum_fn]` now takes `visibility name -> Type`, replacing
  `name: Type`. Methods are private unless a visibility is written.
- **Breaking:** `strutuct!` declarations and fields are private by default.
  Use `#[strutuct(pub)]` to make a branch public, or write visibility on the
  specific declaration or field. The old `public = true/false` option is gone.
- **Breaking:** ordinary enum tuple and struct variants are preserved by default.
  Enable `product_variants` (`prod`) for packed product payloads.
- **Breaking:** `strutuct!` boolean assignments are replaced by bare flags:
  `inclusions = true` becomes `inclusions`; `inclusions = false` becomes
  `no_inclusions`. The same convention applies to `product_variants` and
  `reverse_concat`, including their short aliases.
- **Breaking:** `strutuct!` preserves supplied documentation without adding
  placeholder descriptions to declarations, fields, or variants. Normal
  `missing_docs` lint settings apply; opt out with `undoc` where appropriate.
- The whole-stream syntax extensions accept exclusions directly in their input,
  either through the tagged envelope or the existing inline
  `exclude_macros = (names);` form. The short name `exclude` works in both
  forms and in `expand!`.
- `forward_attributes` preserves an existing parameter envelope before forwarded
  attributes. Place `#[macro_args(...)]` above `#[forward_attributes]`.
- Expanded rustdoc argument references and book chapters; book highlighting uses
  the preprocessor's bundled language support instead of checked-in parser binaries.
- The crate is now dual-licensed under MIT or Apache-2.0.

### Removed

- **Breaking:** `#[excluded_macros]` is removed. Use
  `#[macro_args(exclude_macros = (names))]` or the inline option instead.
- `strutuct!` no longer accepts the unused shared macro-exclusion option.

## 0.10.0 - 2026-10-14

### Added

- `#[forall]` writes the constant part of a member declaration once and repeats
  it over every field or variant: outer attributes, one visibility, the shared
  types, or one group of shared named fields, written in any order. A tuple
  struct whose fields are bare names becomes a braced struct of one shared type,
  the variants of an enum take the shared types as unnamed fields or the shared
  group beside their own named ones, and a created named field is documented
  where the declaration documents none. A prefix forwarded by a declarative
  macro is read like one written directly, including a `vis` fragment that
  matched nothing and a field name arriving as a `ty` fragment.
- Book chapter on `forall`.

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
