# Macros

The crate currently has two kinds of macro.

The first kind generates ordinary Rust from compact input:

- [`discriminated_str`](discriminated_str.md) works on an enum declaration;
- [`strutuct!`](strutuct.md) generates related structs, enums, and constructor
  macros;
- [`qf!`](qf.md) rewrites one Rust type.

The second kind rewrites raw token streams recursively:

- `literally_literal_string!` recognizes the deliberately invalid `@@"text"`;
- `excluded_macros` marks macro invocations whose contents must remain opaque;
- `expand!` loads an out-of-line module and applies one or more rewriting
  macros before rustc parses its body.

Read [How token rewriting behaves](token-rewriting.md) before combining the
second group with attributes or macros that consume their own private syntax.
