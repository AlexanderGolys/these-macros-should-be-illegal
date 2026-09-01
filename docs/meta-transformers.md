# Meta transformers

Meta transformers operate on invocation objects and token-stream structure
rather than assuming that their input is a Rust item or expression.

## Reflecting invocations

`reflect!` accepts two macro paths and an opaque body:

```rust
use these_macros_should_be_illegal::reflect;

macro_rules! add_one {
    ($expression:expr) => { 1 + $expression };
}

macro_rules! double {
    ($expression:expr) => { 2 * $expression };
}

let reflected = reflect!(add_one, double; 3);
assert_eq!(reflected, 8);
```

It performs only the structural reflection

```text
first! { second! { body } }
    ↦ second! { first! { body } }
```

so the example becomes `double! { add_one! { 3 } }`. The inner body is never
parsed by `reflect!`, and both macro paths retain their call-site spans. This is
not an API for forcing another macro to expand inside the current procedural
macro; it constructs the reflected invocation objects and lets rustc expand
them normally afterward.

Internally the implementation is exactly the composition

```text
invoke(second, invoke(first, body))
```

where `invoke` pairs one macro path with one opaque token stream.

## Permuting token trees

`perm!` uses ordinary one-based cycle notation. Separate positions with spaces:
`14` is one integer token, whereas `1 4` denotes two positions.

```text
perm! { ((1 4 3)), a, b, c, d, e }
    -> c, b, d, a, e
```

Cycles compose from right to left. The permutation acts by moving the token tree
at position `i` to position `σ(i)`. Positions beyond the largest index appearing
in any cycle remain fixed, so the same notation implicitly embeds `S_n` into
every `S_{n+k}`.

The implementation stores the destination of each source position only through
the final non-fixed point. Therefore the identity needs no stored entries, a
cycle constructs one finite map, and multiplication is ordinary function
composition:

```text
(sigma * tau)(i) = sigma(tau(i))
```

This representation is also what applies a written cycle product to the token
trees, so the algebra exercised by the implementation tests and the documented
right-to-left expansion rule are the same code path.

Each comma-delimited operand must be exactly one token tree: an identifier,
literal, punctuation token, or one delimited group. Put a multi-token operand in
a group if it should move as one unit.

The expansion is the raw comma-separated stream, deliberately without a tuple,
array, block, or other imposed Rust shape. This makes `perm!` a structural
building block, but its invocation must ultimately occur in a context whose
consumer accepts that output shape.

Run the two complete programs shipped with the crate using
`cargo run --example meta_transformers` and `cargo run --example enum_fn`.
