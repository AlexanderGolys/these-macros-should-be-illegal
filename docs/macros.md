# Macros

The crate currently has three broad kinds of macro.

The first kind generates ordinary Rust from compact input:

- [`callable` and `make_fn!`](callable.md) give objects function-like macro
  syntax while retaining their original types and methods;
- [`discriminated_str`](discriminated_str.md) creates a unique string mapping
  and a literal-selected constructor macro;
- [`enum_fn`](enum_fn.md) turns per-variant expressions into a method;
- [`strutuct!`](strutuct.md), also exported as `emmun!`, generates related
  structs, enums, and constructor macros;
- [`qf!`](qf.md) rewrites one Rust type.

The second kind rewrites raw token streams recursively:

- [`shared_match_arms!`](shared-match-arms.md) duplicates one match-arm RHS
  across independently typed patterns;
- [`forward_attributes`](forward-attributes.md) moves outer attributes behind
  a `;` boundary inside any function-like macro invocation;
- `literally_literal_string!` recognizes the deliberately invalid `@@"text"`;
- `excluded_macros` marks macro invocations whose contents must remain opaque;
- `expand!` loads an out-of-line module and applies one or more rewriting
  macros before rustc parses its body.

Read [How token rewriting behaves](token-rewriting.md) before combining the
second group with attributes or macros that consume their own private syntax.

The third kind transforms macro and token-stream structure itself:

- [`reflect!`](meta-transformers.md#reflecting-invocations) exchanges two
  nested invocation nodes around an opaque body;
- [`perm!`](meta-transformers.md#permuting-token-trees) applies a finite
  permutation to comma-separated token trees while fixing the remaining
  positions.
