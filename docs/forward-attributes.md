# `forward_attributes`

API reference on docs.rs: [`forward_attributes`](https://docs.rs/these-macros-should-be-illegal/latest/these_macros_should_be_illegal/attr.forward_attributes.html).

`forward_attributes` turns ordinary outer attributes into an argument envelope
for any item-position function-like macro. The receiving macro decides what the
attributes mean.

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
#[forward_attributes]
#[derive(Debug, PartialEq)]
strutuct! {
    State { Ready, Waiting }
}

fn main() {
    assert_eq!(State::Ready, State::Ready);
    assert_eq!(format!("{:?}", State::Waiting), "Waiting");
}
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
strutuct! {
    #[derive(Debug, PartialEq)]
    ;
    State { Ready, Waiting }
}

fn main() {
    assert_eq!(State::Ready, State::Ready);
    assert_eq!(format!("{:?}", State::Waiting), "Waiting");
}
```

</div>

</div>

The semicolon distinguishes invocation-wide attributes from attributes that
were already attached to the first object in the macro input. `strutuct!` and
`emmun!` interpret the left side as inherited attributes and configuration;
local attributes on the right retain their branch-specific meaning.

## Why the invocation is enough

A function-like macro invocation in item position is already an
attribute-compatible item shell. `forward_attributes` therefore does not need
to invent a dummy struct: it parses only that shell, removes its outer
attributes, and puts them at the front of the invocation's existing token
group. Rust still leaves the receiving macro's private contents opaque.

This is specifically an item-position bridge. It does not make a macro call in
type, pattern, or nested expression position into a target for an attribute
procedural macro.

## Expansion order

`forward_attributes` must be the first active attribute. An active attribute
written before it may expand or fail before forwarding gets a turn:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>What Rust sees first</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust,compile_fail
#[derive(Debug)]
#[forward_attributes]
strutuct! { TooLate { A, B } }

fn main() {}
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
// `derive` runs before `forward_attributes`
// can move it into the macro input.
compile_error!(
    "derive may only be applied to structs, enums and unions"
);
```

</div>

</div>

Inert attributes such as documentation may precede it and are forwarded too.

## Opaque input

Only the function-like invocation shell is parsed. Its delimited input remains
an opaque token stream and may use any private syntax accepted by the receiving
macro. The input must still be tokenizable and have balanced delimiters.

Macros opt into this convention by parsing:

```text
zero or more outer attributes ; ordinary macro input
```

There is deliberately no universal meaning attached to those attributes.
