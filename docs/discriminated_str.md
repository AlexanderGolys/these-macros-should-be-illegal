# `discriminated_str`

API reference on docs.rs: [`discriminated_str`](https://docs.rs/these-macros-should-be-illegal/latest/these_macros_should_be_illegal/attr.discriminated_str.html).

Assigns every enum variant one unique string literal. The generated method maps
values to strings; a same-name macro maps literals and payloads back to variant
constructors.

```rust,pane="You write"
# #[macro_use] extern crate these_macros_should_be_illegal;
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

```rust,ignore,pane="Roughly expands to"
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

The constructor macro follows the original variant shape:

```rust,pane="You write"
# #[macro_use] extern crate these_macros_should_be_illegal;
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

```rust,ignore,pane="Roughly expands to"
Error!("io", source)
// Error::Io(source)

Error!("parse", offset: 17)
// Error::Parse { offset: 17 }

Error!("unknown")
// Error::Unknown
```

Every variant must have a string literal, and literals must be unique. The
constructor direction deliberately accepts literals rather than runtime
`&str` values: it selects a constructor during macro expansion and delegates
the remaining expressions directly to it. No generated runtime tag type leaks
into the user's namespace.

Generics, lifetimes, visibility, and where clauses remain on the original enum.
`cfg` and `cfg_attr` are retained on the generated forward match arms.
