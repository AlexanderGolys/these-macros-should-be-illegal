# Macro parameters

Function-like macros accept an inline `@macro_args(...);` prefix before
their body. Parameters are ordinary unexpanded tokens.

```rust
use these_macros_should_be_illegal::strutuct;
strutuct! { @macro_args(product_variants); Message Pair(u8, u8) }
```

| Macro | Parameters |
| --- | --- |
| `literally_literal_string!`, `shared_match_arms!` | `exclude_macros = (names)` |
| `strutuct!`, `emmun!` | Root configuration: visibility, `inclusions`, `product_variants`, `reverse_concat`, `allow_missing_docs`, and their `no_` forms |
| `stringify_as!` | One spelling option, such as `snake` or `type` |
| `expand!` | Macro paths and shared preprocessing options |
| Other function-like macros in this crate | Empty parameters |

Existing inline forms remain available. Choose either the tagged prefix or the
inline parameter form for an invocation. A repeated tagged prefix is an error.
Local `#[strutuct(...)]` options override root defaults supplied through the
parameter list. Flags enable their options; `no_inclusions`,
`no_product_variants`, and `no_reverse_concat` disable inherited options.
All flags are disabled by default.

Full names and short aliases are interchangeable:

| Full name | Short name | Negated form | Default |
| --- | --- | --- | --- |
| `inclusions` | `inc` | `no_inclusions` / `no_inc` | Disabled |
| `product_variants` | `prod` | `no_product_variants` / `no_prod` | Disabled |
| `reverse_concat` | `rev` | `no_reverse_concat` / `no_rev` | Disabled |
| `allow_missing_docs` | `undoc` | `no_allow_missing_docs` / `no_undoc` | Disabled |
| `exclude_macros = (names)` | `exclude = (names)` | None | Empty list |

The exclusion alias works in both syntax-rewriting macros and `expand!`,
including existing inline option forms. A full name and its alias refer to
one option; repeating either spelling in the same argument list is an error.


The prefix works in item, expression, and type positions where the receiving
macro is supported.
