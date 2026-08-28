# `qf!`

`qf!` recursively qualifies common unqualified standard-library types. It is
mainly useful in generated code, where relying on imports from the caller would
be impolite.

```rust
use these_macros_should_be_illegal::qf;

type Messages = qf!(Option<Vec<String>>);

let messages: Messages = Some(vec![String::from("hello")]);
assert_eq!(messages.as_ref().map(Vec::len), Some(1));
```

The type above expands to the equivalent of:

```rust
type Messages =
    ::core::option::Option<::std::vec::Vec<::std::string::String>>;
```

The recognized names are:

- `String`, `Box`, and `Vec`;
- `HashMap`, `HashSet`, `BTreeMap`, and `BTreeSet`;
- `Option` and `Arc`.

Only single-segment paths are rewritten. A deliberately qualified path such as
`application::String` is preserved, because writing the qualifier is how the
caller says that this is not `std::string::String`.
