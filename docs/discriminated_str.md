Assigns every enum variant one unique string literal. The generated method maps
values to strings; a same-name macro maps literals and payloads back to variant
constructors.

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::discriminated_str;

#[discriminated_str(name)]
enum Token {
    Ident(String) = "ident",
    End = "end",
}

fn main() {
    let token = Token!("ident", String::from("value"));
    assert_eq!(token.name(), "ident");
    assert!(matches!(Token!("end"), Token::End));
}
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
enum Token {
    Ident(String),
    End,
}

impl Token {
    const fn name(&self) -> &'static str {
        match self {
            Self::Ident(..) => "ident",
            Self::End => "end",
        }
    }
}

// Generated in the macro namespace:
macro_rules! Token {
    ("ident", $value:expr) => { Token::Ident($value) };
    ("end") => { Token::End };
}
```

</div>

</div>

The constructor macro follows the original variant shape:

<div class="highlight-comparison-key">
  <strong>You write</strong>
  <strong>Roughly expands to</strong>
</div>

<div class="highlight-comparison">

<div class="highlight-comparison-pane">

```rust
use these_macros_should_be_illegal::discriminated_str;

#[discriminated_str(code)]
enum Error {
    Io(std::io::Error) = "io",
    Parse { offset: usize } = "parse",
    Unknown = "unknown",
}

fn main() {
    let error = Error!("parse", offset: 17);
    assert_eq!(error.code(), "parse");
}
```

</div>

<div class="highlight-comparison-pane">

```rust,ignore
Error!("io", source)
// Error::Io(source)

Error!("parse", offset: 17)
// Error::Parse { offset: 17 }

Error!("unknown")
// Error::Unknown
```

</div>

</div>

Every variant must have a string literal, and literals must be unique. The
constructor direction deliberately accepts literals rather than runtime
`&str` values: it selects a constructor during macro expansion and delegates
the remaining expressions directly to it. No generated runtime tag type leaks
into the user's namespace.

Generics, lifetimes, visibility, and where clauses remain on the original enum.
`cfg` and `cfg_attr` are retained on the generated forward match arms.
