# How token rewriting behaves

The syntax-rewriting macros operate structurally on token trees rather than by
editing source strings.

## Groups are recursive

Every ordinary parenthesized, braced, and bracketed token group is visited
recursively. A transformation therefore works at any nesting depth without
assuming that its input is a Rust item, expression, type, or even valid Rust
syntax yet.

## Attributes are opaque

Outer and inner attributes are copied as complete opaque regions. Tokens inside
`#[...]` and `#![...]` are never rewritten. This keeps derive helpers, `cfg`
expressions, and other attribute-specific syntax under the control of the
attribute that owns them.

## Excluded macros are opaque

An excluded invocation is copied together with its complete delimited input.
Exclusion is exact, including raw identifiers. Declarative macro definitions
can also be excluded without rewriting their bodies.

Macro exclusions are shared through a private configuration envelope when
several rewriting macros are chained by `expand!`.

## What this does not promise

Token rewriting does not make arbitrary text valid Rust. The final transformed
stream still has to parse and type-check normally. Diagnostics after a large
transformation may also point at the macro invocation rather than at an exact
character in the out-of-line source.
