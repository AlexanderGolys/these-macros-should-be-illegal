//! Experimental procedural macros for compact local DSLs, recursive token
//! rewriting, and transformations of macro structure.
//!
//! The main entry points are [`enum_fn`] for per-variant methods,
//! [`discriminated_str`] for reversible string discriminants, [`strutuct`] for
//! nested algebraic declarations, and [`expand`] for applying recursive syntax
//! extensions to an out-of-line module. Every macro is independent; import only
//! the syntax used by the caller.
//!
//! ```
//! use these_macros_should_be_illegal::enum_fn;
//!
//! #[enum_fn(code: usize)]
//! enum Status {
//!     Ready = 200,
//!     Missing,
//! }
//!
//! assert_eq!(Status::Ready.code(), Some(200));
//! assert_eq!(Status::Missing.code(), None);
//! ```
//!
//! The [crate repository](https://github.com/AlexanderGolys/these-macros-should-be-illegal)
//! contains the complete book and runnable examples.

/// Whole-stream grammar extensions applied recursively before parsing.
mod flust;
/// Reusable plumbing for composing and configuring macros.
mod helpers;
/// Shape-aware local syntax extensions and replacement grammars.
mod local;
/// Macros which transform macro invocations and token-stream structure.
mod meta;
/// Small conveniences over otherwise ordinary Rust syntax.
mod rast;

use flust::{literally_literal_string_impl, shared_match_arms_impl};
use helpers::{excluded_macros_impl, expand_impl, forward_attributes_impl};
use local::{
    StringCase, callable_impl, discriminated_str_impl, enum_fn_impl, make_fn_impl, stringify_case,
    stringify_type_impl, strutuct_impl,
};
use meta::{perm_impl, reflect_impl};
use proc_macro::TokenStream;
use rast::qualify_common_impl;

/// Rewrites literal strings in exactly the supplied token stream.
///
/// ```
/// use these_macros_should_be_illegal::literally_literal_string;
///
/// let owned: String = literally_literal_string!(@@"hello");
/// assert_eq!(owned, "hello");
/// ```
#[proc_macro]
pub fn literally_literal_string(input: TokenStream) -> TokenStream {
    literally_literal_string_impl(input.into()).into()
}

/// Clones a shared match-arm RHS for alternatives separated by `||`.
///
/// Parenthesize `(pattern if guard)` when one alternative has its own guard.
///
/// ```
/// use these_macros_should_be_illegal::shared_match_arms;
///
/// enum Value {
///     Number(u32),
///     Character(char),
/// }
///
/// let value = Value::Number(3);
/// let text = shared_match_arms! {
///     match value {
///         Value::Number(value) || Value::Character(value) => value.to_string(),
///     }
/// };
///
/// assert_eq!(text, "3");
/// ```
#[proc_macro]
pub fn shared_match_arms(input: TokenStream) -> TokenStream {
    shared_match_arms_impl(input.into()).into()
}

/// Qualifies common unqualified standard-library types inside one Rust type.
///
/// Nested common types are qualified recursively, while an already qualified
/// path is preserved deliberately.
///
/// ```
/// use these_macros_should_be_illegal::qf;
///
/// let value: qf!(Option<Vec<String>>) = Some(vec![String::from("less punctuation")]);
/// assert!(value.is_some());
/// ```
#[proc_macro]
pub fn qf(input: TokenStream) -> TokenStream {
    qualify_common_impl(input.into()).into()
}

/// Converts an identifier or string literal to `camelCase`.
///
/// ```
/// use these_macros_should_be_illegal::stringify_camel_case;
/// assert_eq!(stringify_camel_case!(some_HTTP_server), "someHttpServer");
/// ```
#[proc_macro]
pub fn stringify_camel_case(input: TokenStream) -> TokenStream {
    stringify_case(input.into(), StringCase::Camel).into()
}

/// Converts an identifier or string literal to `PascalCase`.
///
/// This case is also commonly called `UpperCamelCase`.
///
/// ```
/// use these_macros_should_be_illegal::stringify_pascal_case;
/// assert_eq!(stringify_pascal_case!(some_HTTP_server), "SomeHttpServer");
/// ```
#[proc_macro]
pub fn stringify_pascal_case(input: TokenStream) -> TokenStream {
    stringify_case(input.into(), StringCase::Pascal).into()
}

/// Converts an identifier or string literal to `snake_case`.
///
/// ```
/// use these_macros_should_be_illegal::stringify_snake_case;
/// assert_eq!(stringify_snake_case!(SomeHTTPServer), "some_http_server");
/// ```
#[proc_macro]
pub fn stringify_snake_case(input: TokenStream) -> TokenStream {
    stringify_case(input.into(), StringCase::Snake).into()
}

/// Converts an identifier or string literal to `kebab-case`.
///
/// ```
/// use these_macros_should_be_illegal::stringify_kebab_case;
/// assert_eq!(stringify_kebab_case!(SomeHTTPServer), "some-http-server");
/// ```
#[proc_macro]
pub fn stringify_kebab_case(input: TokenStream) -> TokenStream {
    stringify_case(input.into(), StringCase::Kebab).into()
}

/// Converts an identifier or string literal to `SCREAMING_SNAKE_CASE`.
///
/// ```
/// use these_macros_should_be_illegal::stringify_screaming_snake_case;
/// assert_eq!(stringify_screaming_snake_case!(SomeHTTPServer), "SOME_HTTP_SERVER");
/// ```
#[proc_macro]
pub fn stringify_screaming_snake_case(input: TokenStream) -> TokenStream {
    stringify_case(input.into(), StringCase::ScreamingSnake).into()
}

/// Converts an identifier or string literal to `lowercase` without separators.
///
/// ```
/// use these_macros_should_be_illegal::stringify_lowercase;
/// assert_eq!(stringify_lowercase!(Some_HTTP_Server), "some_http_server");
/// ```
#[proc_macro]
pub fn stringify_lowercase(input: TokenStream) -> TokenStream {
    stringify_case(input.into(), StringCase::Lower).into()
}

/// Converts an identifier or string literal to `UPPERCASE` without separators.
///
/// ```
/// use these_macros_should_be_illegal::stringify_uppercase;
/// assert_eq!(stringify_uppercase!(Some_HTTP_Server), "SOME_HTTP_SERVER");
/// ```
#[proc_macro]
pub fn stringify_uppercase(input: TokenStream) -> TokenStream {
    stringify_case(input.into(), StringCase::Upper).into()
}

/// Produces a canonical, collision-safe string name for one Rust type.
///
/// The result begins with `type:`, which is deliberately illegal in Rust
/// identifiers. Formatting differences are removed while type structure is
/// preserved: `& 'a mut Vec < Option < T > >` becomes
/// `"type:&'a mut Vec<Option<T>>"`.
///
/// ```
/// use these_macros_should_be_illegal::stringify_type;
/// assert_eq!(
///     stringify_type!(fn((u8, u16), *const [u8; 4]) -> bool),
///     "type:fn((u8,u16),*const[u8;4])->bool",
/// );
/// ```
#[proc_macro]
pub fn stringify_type(input: TokenStream) -> TokenStream {
    stringify_type_impl(input.into()).into()
}

/// Permutes comma-separated token trees using standard cycle notation.
///
/// Cycles compose from right to left and positions are one-based. Token trees
/// beyond the largest mentioned position remain fixed. Spaces are required
/// between positions because `14` is one integer token while `1 4` is two.
///
/// ```text
/// perm! { ((1 4 3)), a, b, c, d, e }
/// // expands to the token stream: c, b, d, a, e
/// ```
///
/// An empty cycle product demonstrates the identity in an ordinary expression
/// context; nontrivial products deliberately return a raw comma-separated
/// stream for structural consumers.
///
/// ```
/// use these_macros_should_be_illegal::perm;
/// let fixed: &str = perm!((), "fixed");
/// assert_eq!(fixed.len(), 5);
/// ```
#[proc_macro]
pub fn perm(input: TokenStream) -> TokenStream {
    perm_impl(input.into()).into()
}

/// Reflects two macro invocation objects around an opaque token-stream body.
///
/// `reflect!(first, second; body)` constructs
/// `second! { first! { body } }`. Consequently `second` expands before
/// `first`, reversing the expansion order of `first! { second! { body } }`.
///
/// ```
/// use these_macros_should_be_illegal::reflect;
/// macro_rules! add_one { ($value:expr) => { 1 + $value }; }
/// macro_rules! double { ($value:expr) => { 2 * $value }; }
/// assert_eq!(reflect!(add_one, double; 3), 8);
/// ```
#[proc_macro]
pub fn reflect(input: TokenStream) -> TokenStream {
    reflect_impl(input.into()).into()
}

#[doc = include_str!("../docs/callable.md")]
#[proc_macro_attribute]
pub fn callable(arguments: TokenStream, item: TokenStream) -> TokenStream {
    callable_impl(arguments.into(), item.into()).into()
}

/// Creates a local value and a same-name macro providing function-like calls.
///
/// The value's type must expose the method generated by [`callable`]. The
/// accepted syntax is `[mut] name = expression`.
///
/// ```
/// use these_macros_should_be_illegal::{callable, make_fn};
///
/// #[callable(apply)]
/// trait Action {
///     fn apply(&self, point: usize) -> usize;
/// }
///
/// struct Shift(usize);
///
/// impl Action for Shift {
///     fn apply(&self, point: usize) -> usize {
///         point + self.0
///     }
/// }
///
/// make_fn!(sigma = Shift(3));
/// assert_eq!(sigma!(2), 5);
/// ```
#[proc_macro]
pub fn make_fn(input: TokenStream) -> TokenStream {
    make_fn_impl(input.into()).into()
}

#[doc = include_str!("../docs/discriminated_str.md")]
#[proc_macro_attribute]
pub fn discriminated_str(arguments: TokenStream, item: TokenStream) -> TokenStream {
    discriminated_str_impl(arguments.into(), item.into()).into()
}

#[doc = include_str!("../docs/enum_fn.md")]
#[proc_macro_attribute]
pub fn enum_fn(arguments: TokenStream, item: TokenStream) -> TokenStream {
    enum_fn_impl(arguments.into(), item.into()).into()
}

/// Prevents transformations from descending into the listed macro invocations.
///
/// ```
/// use these_macros_should_be_illegal::{excluded_macros, literally_literal_string};
/// macro_rules! raw_tokens { ($($tokens:tt)*) => { "borrowed" }; }
///
/// #[excluded_macros(raw_tokens)]
/// literally_literal_string! {
///     fn value() -> &'static str { raw_tokens!(@@"untouched") }
/// }
///
/// fn main() {
///     assert_eq!(value(), "borrowed");
/// }
/// ```
#[proc_macro_attribute]
pub fn excluded_macros(arguments: TokenStream, item: TokenStream) -> TokenStream {
    excluded_macros_impl(arguments.into(), item.into()).into()
}

/// Loads an out-of-line module and injects function-like macros around its body.
///
/// The declaration points to a real source file, so this outline is ignored in
/// rustdoc and exercised by the integration fixtures instead.
///
/// ```rust,ignore
/// use these_macros_should_be_illegal::{expand, literally_literal_string};
///
/// expand!(literally_literal_string; mod experiments);
/// // `experiments.rs` may now contain the extended token syntax.
/// ```
#[proc_macro]
pub fn expand(input: TokenStream) -> TokenStream {
    expand_impl(input.into()).into()
}

#[doc = include_str!("../docs/forward-attributes.md")]
#[proc_macro_attribute]
pub fn forward_attributes(arguments: TokenStream, item: TokenStream) -> TokenStream {
    forward_attributes_impl(arguments.into(), item.into()).into()
}

#[doc = include_str!("../docs/strutuct.md")]
#[proc_macro]
pub fn strutuct(input: TokenStream) -> TokenStream {
    strutuct_impl(input.into()).into()
}

/// Expands exactly like [`strutuct!`](strutuct).
///
/// ```
/// use these_macros_should_be_illegal::emmun;
/// emmun! { State { Ready, Waiting } }
/// assert!(matches!(State::Ready, State::Ready));
/// ```
#[proc_macro]
pub fn emmun(input: TokenStream) -> TokenStream {
    strutuct_impl(input.into()).into()
}
