# Macros

The public macros fall into five families. The distinction is about what each
macro is allowed to assume about its input, not about which procedural-macro
kind happens to implement it.

## Structured local syntax

These macros parse a known local shape and generate ordinary Rust:

- [`callable` and `make_fn!`](callable.md) give objects function-like macro
  syntax while retaining their original types and methods;
- [`discriminated_str`](discriminated_str.md) creates a unique string mapping
  and a literal-selected constructor macro;
- [`enum_fn`](enum_fn.md) turns per-variant expressions into a method;
- [`strutuct!`](strutuct.md), also exported as `emmun!`, generates related
  structs, enums, and constructor macros;
- [`overload_op`](overload-op.md) repeats one operator written for references
  across its owned operand forms and its augmented assignment;
- [`complete_ops`](complete-ops.md) adds the operators that follow from the
  ones a type already has, such as subtraction from addition and negation.

## Ordinary Rust fragments and names

These conveniences accept one ordinary Rust fragment or name:

- [`qf!`](qf.md) recursively qualifies common paths in one Rust type;
- [`stringify_as!`](stringification.md) converts one identifier or string
  literal to a selected [conventional case](stringification.md#name-cases), or
  emits the compact, structure-preserving string name of
  [one Rust type](stringification.md#rust-types).

## Recursive whole-stream extensions

These macros rewrite arbitrary token trees recursively without assuming that
the complete input is an item or expression:

- [`shared_match_arms!`](shared-match-arms.md) duplicates one match-arm RHS
  across independently typed patterns;
- [`literally_literal_string!`](literal-syntax.md#literally_literal_string)
  recognizes the deliberately invalid `@@"text"`.

Read [How token rewriting behaves](token-rewriting.md) before combining the
whole-stream extensions with attributes or macros that consume private syntax.

## Composition helpers

These helpers transport or protect token streams for another macro:

- [`forward_attributes`](forward-attributes.md) moves an item-position macro
  invocation's outer attributes behind a `;` boundary in its opaque input;
- [`excluded_macros`](literal-syntax.md#excluding-macro-inputs) marks macro
  invocations whose contents a recursive transformation must leave opaque;
- [`expand!`](literal-syntax.md#expand) loads one out-of-line module and wraps
  its body in one or more whole-stream transformations before rustc parses it.

## Meta-transformers

These macros transform macro and token-stream structure itself:

- [`reflect!`](meta-transformers.md#reflecting-invocations) exchanges two
  nested invocation nodes around an opaque body;
- [`perm!`](meta-transformers.md#permuting-token-trees) applies a finite
  permutation to comma-separated token trees while fixing the remaining
  positions.
