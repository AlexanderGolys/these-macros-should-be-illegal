# `forward_attributes`

`forward_attributes` turns ordinary outer attributes into an argument envelope
for any item-position function-like macro. The receiving macro decides what the
attributes mean.

```rust
# #![allow(clippy::needless_doctest_main)]
use these_macros_should_be_illegal::{forward_attributes, strutuct};

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

The forwarding step above is structurally equivalent to:

```text
strutuct! {
    #[derive(Debug, PartialEq)]
    ;
    State { Ready, Waiting }
}
```

The semicolon distinguishes invocation-wide attributes from attributes that
were already attached to the first object in the macro input. `strutuct!` and
`emmun!` interpret the left side as inherited attributes and configuration;
local attributes on the right retain their branch-specific meaning.

## Expansion order

`forward_attributes` must be the first active attribute. An active attribute
written before it may expand or fail before forwarding gets a turn:

```rust,compile_fail
use these_macros_should_be_illegal::{forward_attributes, strutuct};

#[derive(Debug)]
#[forward_attributes]
strutuct! { TooLate { A, B } }

fn main() {}
```

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
