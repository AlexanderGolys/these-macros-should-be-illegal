# These Macros Should Be Illegal

This crate is a small collection of experimental Rust macros for deleting
boilerplate and trying syntax that ordinary Rust—quite reasonably—does not
accept. The useful bits come first; the cursed implementation details are here
when you actually need them.

```sh
cargo add these-macros-should-be-illegal
```

Import whichever macro you use:

```rust
use these_macros_should_be_illegal::{
    discriminated_str, emmun, enum_fn, forward_attributes, qf, shared_match_arms,
    strutuct,
};
```

## Pick your crime

- [`discriminated_str`](discriminated_str.md) assigns each variant a unique
  string and generates a literal-selected delegating constructor.
- [`enum_fn`](enum_fn.md) generates a method from inline per-variant match-arm
  expressions.
- [`strutuct!`](strutuct.md), with `emmun!` as an exact alias, declares a small
  family of nested algebraic types in one place and generates constructor macros
  for them.
- [`forward_attributes`](forward-attributes.md) makes outer attributes usable as
  function-like macro configuration.
- [`qf!`](qf.md) gives common standard-library types their fully qualified
  paths inside generated code.
- [`shared_match_arms!`](shared-match-arms.md) shares an RHS between patterns
  whose bindings have different concrete types.
- [`literally_literal_string!` and `expand!`](literal-syntax.md) let token
  rewriting handle syntax that Rust cannot parse as an ordinary source file.

## Stability

This is a personal stable-Rust macro laboratory. The APIs are intentionally
small and experimental, and may evolve whenever a better crime presents
itself. Releases are available from
[crates.io](https://crates.io/crates/these-macros-should-be-illegal), and the
ordinary API reference lives on
[docs.rs](https://docs.rs/these-macros-should-be-illegal).
