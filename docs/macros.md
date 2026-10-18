# Macros

The public macros fall into four families. The distinction is about what each
macro is allowed to assume about its input, not about which procedural-macro
kind happens to implement it.

## Generators

These macros parse a known local shape and declare types, together with the
structure built around them:

- [`callable`](callable.md) marks the trait method that gives an object
  function-like macro syntax, retaining its original type and methods;
- [`discriminated_str`](discriminated_str.md) creates a unique string mapping
  and a literal-selected constructor macro;
- [`enum_fn`](enum_fn.md) turns per-variant expressions into a method;
- [`forall`](forall.md) writes the constant part of a field or variant
  declaration once and repeats it over all of them;
- [`Log`](log.md), a derive, generates the enum of a struct's field names and
  the indexing operator evaluating one of them on a value;
- [`strutuct!`](strutuct.md), also exported as `emmun!`, generates related
  structs, enums, and constructor macros;
- [`overload_op`](overload-op.md) repeats one operator written for references
  across its owned operand forms and its augmented assignment;
- [`complete_ops`](complete-ops.md) adds the operators that follow from the
  ones a type already has, such as subtraction from addition and negation.

## Whole-stream syntax extensions

These macros rewrite arbitrary token trees recursively without assuming that
the complete input is an item or expression. Each takes an optional leading
`exclude_macros = (names);` option, ended by a semicolon, naming the macro
invocations it must leave opaque:

- [`shared_match_arms!`](shared-match-arms.md) duplicates one match-arm RHS
  across independently typed patterns;
- [`literally_literal_string!`](literal-syntax.md#literally_literal_string)
  recognizes the deliberately invalid `@@"text"`.

Read [How token rewriting behaves](token-rewriting.md) before combining the
whole-stream extensions with attributes or macros that consume private syntax.

## Meta-transformers

These macros transform other macros, and the token streams passed to them:

- [`expand!`](literal-syntax.md#expand) loads one out-of-line module and wraps
  its body in one or more whole-stream transformations before rustc parses it;
- [`forward_attributes`](forward-attributes.md) moves an item-position macro
  invocation's outer attributes behind a `;` boundary in its opaque input;
- [`make_fn!`](callable.md) binds a value together with the same-name macro
  that injects its arguments into the method [`callable`](callable.md) named;
- [`reflect!`](meta-transformers.md#reflecting-invocations) exchanges two
  nested invocation nodes around an opaque body;
- [`perm!`](meta-transformers.md#permuting-token-trees) applies a finite
  permutation to comma-separated token trees while fixing the remaining
  positions.

## Quality of life

These conveniences accept one ordinary Rust fragment or name:

- [`qf!`](qf.md) recursively qualifies common paths in one Rust type;
- [`stringify_as!`](stringification.md) converts one identifier or string
  literal to a selected [conventional case](stringification.md#name-cases), or
  emits the compact, structure-preserving string name of
  [one Rust type](stringification.md#rust-types).
