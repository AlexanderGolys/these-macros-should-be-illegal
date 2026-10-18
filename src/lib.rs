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
//! # use these_macros_should_be_illegal::enum_fn;
//!
//! #[enum_fn(code -> usize)]
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

use proc_macro::TokenStream;

mod generate;
mod helpers;
mod meta;
mod qol;
mod sand;

// Generators of type declarations and the structure around them.

/// Gives one trait method a structural alias that [`make_fn!`](make_fn) calls through.
///
/// No shared callable trait is imposed: the attribute only marks which method of
/// a user-owned trait the generated same-name macro forwards its arguments to.
///
/// # Examples
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
#[proc_macro_attribute]
pub fn callable(arguments: TokenStream, item: TokenStream) -> TokenStream {
    generate::callable::callable(arguments.into(), item.into()).into()
}

/// Adds the operators that follow from the ones a type already implements.
///
/// `Sub` completes subtraction from addition and negation; `Neg = <scalar>`
/// completes negation from multiplication by that scalar.
///
/// # Examples
///
/// ```
/// use std::ops::{Add, AddAssign, Neg};
/// use these_macros_should_be_illegal::complete_ops;
///
/// #[derive(Debug, PartialEq)]
/// #[complete_ops(Sub)]
/// struct Vector(f64);
///
/// impl Add for Vector {
///     type Output = Vector;
///
///     fn add(self, rhs: Vector) -> Vector {
///         Vector(self.0 + rhs.0)
///     }
/// }
///
/// impl AddAssign for Vector {
///     fn add_assign(&mut self, rhs: Vector) {
///         self.0 += rhs.0;
///     }
/// }
///
/// impl Neg for Vector {
///     type Output = Vector;
///
///     fn neg(self) -> Vector {
///         Vector(-self.0)
///     }
/// }
///
/// assert_eq!(Vector(5.0) - Vector(2.0), Vector(3.0));
/// ```
#[proc_macro_attribute]
pub fn complete_ops(arguments: TokenStream, item: TokenStream) -> TokenStream {
    generate::complete_ops::complete_ops(arguments.into(), item.into()).into()
}

/// Assigns every enum variant one unique string literal.
///
/// The named method maps a value to its literal, and a same-name macro maps a
/// literal and payload back to the variant constructor.
///
/// # Examples
///
/// ```
/// use these_macros_should_be_illegal::discriminated_str;
///
/// #[discriminated_str(name)]
/// enum Token {
///     Ident(String) = "ident",
///     End = "end",
/// }
///
/// fn main() {
///     let token = Token!("ident", String::from("value"));
///     assert_eq!(token.name(), "ident");
///     assert!(matches!(Token!("end"), Token::End));
/// }
/// ```
#[proc_macro_attribute]
pub fn discriminated_str(arguments: TokenStream, item: TokenStream) -> TokenStream {
    generate::discriminated_str::discriminated_str(arguments.into(), item.into()).into()
}

/// Generates an enum method from per-variant expressions written beside the variants.
///
/// The argument is the method signature, `vis name -> Type`; without a
/// visibility the method is private, as in ordinary Rust. The expression after
/// each variant becomes that match arm's value.
///
/// # Examples
///
/// ```
/// use these_macros_should_be_illegal::enum_fn;
///
/// #[enum_fn(description -> &'static str)]
/// enum Action {
///     Quit = "quit the application",
///     Submit = "evaluate the input",
/// }
///
/// const QUIT: &str = Action::Quit.description();
/// assert_eq!(QUIT, "quit the application");
/// ```
#[proc_macro_attribute]
pub fn enum_fn(arguments: TokenStream, item: TokenStream) -> TokenStream {
    generate::enum_fn::enum_fn(arguments.into(), item.into()).into()
}

/// Distributes constant outer attributes, visibility or types over all fields or variants.
///
/// The arguments are written like the beginning of one member declaration,
/// `#[attribute] pub Type`, and are repeated on every member of the annotated
/// item: on each field of a struct or union, and on each variant of an enum.
/// Each part of the prefix is recognized by the syntax that only it has, so
/// the parts may be written in any order. Shared attributes are added in front
/// of the ones a member already carries, and a member keeps its own visibility
/// where it declares one.
///
/// Every item kind takes the part of the prefix its members accept:
///
/// - a braced struct and a union take attributes and one visibility, since
///   their fields already name the type they hold;
/// - a tuple struct takes attributes, one visibility and one shared type, and
///   then its fields are written as bare names: `struct S(a, b)` becomes a
///   braced struct whose fields `a` and `b` hold that type;
/// - an enum takes attributes and either any number of types, appended to its
///   variants as unnamed fields, or one braced group `{ name: Type, ... }` of
///   named fields added to its variants; a variant takes no visibility.
///
/// A named field created from the prefix is documented where the declaration
/// documents none, which keeps the generated braced struct usable under
/// `#![deny(missing_docs)]`.
///
/// A prefix forwarded by a declarative macro is read like one written
/// directly: `#[forall($(#[$attribute])* $visibility $shared)]` takes its
/// parts from `meta`, `vis` and `ty` fragments, including a `vis` fragment
/// that matched nothing, and a field name may be forwarded as a `ty` fragment.
///
/// # Examples
///
/// ```
/// use these_macros_should_be_illegal::forall;
///
/// #[forall(pub(crate))]
/// pub struct Pixel(u8, u8, u8);
///
/// #[forall(pub, f32)]
/// struct Point(x, y);
///
/// #[forall(u16, String)]
/// enum Response { Redirect, Error }
///
/// #[forall(#[doc = "One stage of the pipeline."])]
/// enum Stage { Parse, Lower }
///
/// let point = Point { x: 0.25, y: 0.75 };
/// let response = Response::Error(404, String::from("missing"));
///
/// assert_eq!(point.x + point.y, 1.0);
/// assert!(matches!(response, Response::Error(404, _)));
/// assert!(matches!(Stage::Lower, Stage::Lower));
/// ```
#[proc_macro_attribute]
pub fn forall(arguments: TokenStream, item: TokenStream) -> TokenStream {
    generate::forall::forall(arguments.into(), item.into()).into()
}

/// Generates the enum of the field names of a struct, and its indexing operator.
///
/// A struct is a product over its field names: `S = ∏_{e ∈ E} X_e`, where `E` is
/// the enum of those names. This derive generates that index set, touching
/// nothing of the struct it reads. Where every field holds one type `X`, the
/// product collapses to the exponential `S = X^E = Hom(E, X)`, and evaluating a
/// field name on a value, `E × S → X`, is exactly an
/// [`Index`](core::ops::Index) implementation, generated together with its
/// [`IndexMut`](core::ops::IndexMut) counterpart.
///
/// Each field name becomes one unit variant spelled exactly as the field, so the
/// enum is the set of field names and nothing else; it allows
/// `non_camel_case_types`, since a field spelled to read as a variant is the
/// point. The enum is named `Log` and the struct's name, and takes the struct's
/// visibility. The `log` helper attribute is the heading of its declaration, and
/// names it otherwise: `#[log(Axis)]`, `#[log(pub enum Axis)]`,
/// `#[log(#[derive(Default)] Axis)]`.
///
/// Without a written `derive`, the enum derives `Clone`, `Copy`, `PartialEq`,
/// `Eq`, `Hash` and `Debug`, which an index of a struct is expected to have.
/// Conditional compilation written on a field is repeated on its variant, and a
/// variant is documented where the field it names is not. Fields holding
/// different types generate the index set alone, since a genuine product has no
/// one type to evaluate to, and a struct without fields generates the
/// uninhabited enum: an empty product is a single point.
///
/// The fields are read as the struct declares them, so a tuple struct is
/// rejected: its positions are already indexed by position, and nothing names
/// them. An attribute macro on the struct runs before any derive, which is how
/// [`forall`](macro@crate::forall) writes the fields this one reads.
///
/// # Examples
///
/// ```
/// use these_macros_should_be_illegal::Log;
///
/// # #[derive(Clone, Copy, Debug, PartialEq)]
/// # pub struct Style(u8);
/// #[derive(Log)]
/// #[allow(non_snake_case)]
/// pub struct BaseGroups {
///     /// Style of a keyword.
///     pub Keyword: Style,
///     /// Style of a function name.
///     pub Function: Style,
///     /// Style of the background.
///     pub Bg: Style,
/// }
///
/// let mut groups = BaseGroups {
///     Keyword: Style(1),
///     Function: Style(2),
///     Bg: Style(3),
/// };
/// groups[LogBaseGroups::Bg] = Style(4);
///
/// assert_eq!(groups[LogBaseGroups::Keyword], Style(1));
/// assert_eq!(groups[LogBaseGroups::Bg], Style(4));
/// ```
///
/// ```
/// use these_macros_should_be_illegal::{Log, forall};
///
/// # #[derive(Clone, Copy, Debug, PartialEq)]
/// # pub struct Style(u8);
/// /// Styles of the groups a theme is written in.
/// #[forall(#[doc = "Style of one group."] pub, Style)]
/// #[derive(Log)]
/// #[log(pub Group)]
/// #[allow(non_snake_case)]
/// pub struct Theme(Keyword, Function, Bg);
///
/// let theme = Theme {
///     Keyword: Style(1),
///     Function: Style(2),
///     Bg: Style(3),
/// };
///
/// assert_eq!(theme[Group::Function], Style(2));
/// ```
#[proc_macro_derive(Log, attributes(log))]
pub fn log(item: TokenStream) -> TokenStream {
    generate::log::log(item.into()).into()
}

/// Repeats an operator implemented for references across its owned forms.
///
/// From `impl Op<&Rhs> for &Lhs` it generates `Op<&Rhs>` and `Op<Rhs>` for `Lhs`,
/// and, when the result is `Lhs` itself, both `OpAssign` impls. Every generated
/// impl delegates to the attributed one; nothing is cloned.
///
/// # Examples
///
/// ```
/// use std::ops::Add;
/// use these_macros_should_be_illegal::overload_op;
///
/// #[derive(Debug, PartialEq)]
/// struct Vector(f64, f64);
///
/// #[overload_op]
/// impl Add<&Vector> for &Vector {
///     type Output = Vector;
///
///     fn add(self, rhs: &Vector) -> Vector {
///         Vector(self.0 + rhs.0, self.1 + rhs.1)
///     }
/// }
///
/// let mut total = Vector(1.0, 2.0) + Vector(3.0, 4.0);
/// total += &Vector(1.0, 1.0);
/// assert_eq!(total, Vector(5.0, 7.0));
/// ```
#[proc_macro_attribute]
pub fn overload_op(arguments: TokenStream, item: TokenStream) -> TokenStream {
    generate::overload_op::overload_op(arguments.into(), item.into()).into()
}

/// Declares a family of nested types in place and hoists them into ordinary items.
///
/// Braced bodies become structs or enums by shape and parenthesized bodies
/// become tuple structs; every generated declaration is private unless a
/// visibility says otherwise, and each gets a same-name constructor macro.
/// [`emmun!`](emmun) is an exact alias.
///
/// # Examples
///
/// ```
/// # use these_macros_should_be_illegal::strutuct;
///
/// strutuct!{ Request
///     method: Method { Get, Post },
///     body: String?,
/// }
///
/// let request = Request { method: RequestMethod::Get, body: None };
/// assert!(matches!(request.method, RequestMethod::Get));
/// ```
///
/// A visibility written before a field or declaration applies only there,
/// while one written in `#[strutuct(...)]` becomes the default for the whole
/// branch below it:
///
/// ```
/// # use these_macros_should_be_illegal::strutuct;
///
/// mod api {
///     # use these_macros_should_be_illegal::strutuct;
///     strutuct!{
///         #[strutuct(pub)]
///         struct Request { method: Method { Get, Post } }
///     }
/// }
///
/// let request = api::Request { method: api::RequestMethod::Get };
/// assert!(matches!(request.method, api::RequestMethod::Get));
/// ```
#[proc_macro]
pub fn strutuct(input: TokenStream) -> TokenStream {
    generate::strutuct::strutuct(input.into()).into()
}

/// An exact alias of [`strutuct!`](strutuct), for declarations read as enums.
#[proc_macro]
pub fn emmun(input: TokenStream) -> TokenStream {
    generate::strutuct::strutuct(input.into()).into()
}

// Low-level syntax extensions rewriting a whole token stream.

/// Rewrites literal strings in exactly the supplied token stream.
///
/// An optional leading `exclude_macros = (names);` option keeps the invocations
/// of the listed macros opaque, so the rewriting never descends into them.
///
/// # Examples
///
/// ```
/// # use these_macros_should_be_illegal::literally_literal_string;
/// literally_literal_string! {
///     let literal_string: String = @@"hello";
///     let usual_string: String = "hello".to_string();
///     assert_eq!(literal_string, usual_string);
/// }
/// ```
///
/// ```
/// # use these_macros_should_be_illegal::literally_literal_string;
/// macro_rules! raw_tokens { ($($tokens:tt)*) => { "borrowed" }; }
///
/// literally_literal_string! {
///     exclude_macros = (raw_tokens);
///     fn value() -> &'static str { raw_tokens!(@@"untouched") }
/// }
///
/// fn main() {
///     assert_eq!(value(), "borrowed");
/// }
/// ```
#[proc_macro]
pub fn literally_literal_string(input: TokenStream) -> TokenStream {
    sand::literally_literal_string::literally_literal_string(input.into()).into()
}

/// Clones a shared match-arm RHS for alternatives separated by `||`.
///
/// Parenthesize `(pattern if guard)` when one alternative has its own guard. An
/// optional leading `exclude_macros = (names);` option keeps the invocations of
/// the listed macros opaque, so the rewriting never descends into them.
///
/// # Examples
///
/// ```
/// # use these_macros_should_be_illegal::shared_match_arms;
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
    sand::shared_match_arms::shared_match_arms(input.into()).into()
}

// Transformers of other macros and of the token streams passed to them.

/// Loads an out-of-line module and injects function-like macros around its body.
///
/// The module file is found as for an ordinary `mod` declaration, or through a
/// `#[path]` attribute, and may contain syntax that only the injected macros
/// accept. Here it uses `@@"..."` owned string literals:
///
/// # Examples
///
/// ```
/// use these_macros_should_be_illegal::expand;
///
/// // The macro runs inside the generated module, so name it by full path.
/// expand!(
///     these_macros_should_be_illegal::literally_literal_string;
///     #[path = "../tests/fixtures/literally_literal_string_module.rs"]
///     mod experiments;
/// );
///
/// fn main() {
///     let owned: String = experiments::owned_string();
///     assert_eq!(owned, "loaded outside Rust");
/// }
/// ```
#[proc_macro]
pub fn expand(input: TokenStream) -> TokenStream {
    meta::expand::expand(input.into()).into()
}

/// Moves the outer attributes of an item-position macro invocation into its input.
///
/// The receiving function-like macro then decides what the attributes mean.
///
/// # Examples
///
/// ```
/// use these_macros_should_be_illegal::{forward_attributes, strutuct};
///
/// #[forward_attributes]
/// #[derive(Debug, PartialEq)]
/// strutuct! {
///     State { Ready, Waiting }
/// }
///
/// fn main() {
///     assert_eq!(format!("{:?}", State::Waiting), "Waiting");
/// }
/// ```
#[proc_macro_attribute]
pub fn forward_attributes(arguments: TokenStream, item: TokenStream) -> TokenStream {
    meta::forward_attributes::forward_attributes(arguments.into(), item.into()).into()
}

/// Creates a local value and a same-name macro providing function-like calls.
///
/// The value's type must expose the method generated by [`callable`]. The
/// accepted syntax is `[mut] name = expression`.
///
/// # Examples
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
    meta::make_fn::make_fn(input.into()).into()
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
/// # Examples
///
/// ```
/// # use these_macros_should_be_illegal::perm;
/// let fixed: &str = perm!((), "fixed");
/// assert_eq!(fixed.len(), 5);
/// ```
#[proc_macro]
pub fn perm(input: TokenStream) -> TokenStream {
    meta::perm::perm(input.into()).into()
}

/// Reflects two macro invocation objects around an opaque token-stream body.
///
/// `reflect!(first, second; body)` constructs
/// `second! { first! { body } }`. Consequently `second` expands before
/// `first`, reversing the expansion order of `first! { second! { body } }`.
///
/// # Examples
///
/// ```
/// # use these_macros_should_be_illegal::reflect;
/// macro_rules! add_one { ($value:expr) => { 1 + $value }; }
/// macro_rules! double { ($value:expr) => { 2 * $value }; }
/// assert_eq!(reflect!(add_one, double; 3), 8);
/// ```
#[proc_macro]
pub fn reflect(input: TokenStream) -> TokenStream {
    meta::reflect::reflect(input.into()).into()
}

// Remaining conveniences over ordinary Rust syntax.

/// Qualifies common unqualified standard-library types inside one Rust type.
///
/// Nested common types are qualified recursively, while an already qualified
/// path is preserved deliberately.
///
/// # Examples
///
/// ```
/// # use these_macros_should_be_illegal::qf;
/// let value: qf!(Option<Vec<String>>) = Some(vec![String::from("less punctuation")]);
/// assert!(value.is_some());
/// ```
#[proc_macro]
pub fn qf(input: TokenStream) -> TokenStream {
    qol::qf::qf(input.into()).into()
}

/// Spells one name in a conventional case, or one type canonically, as a string literal.
///
/// The option before `;` selects the spelling: `camel`, `pascal`, `snake`, `kebab`,
/// `screaming_snake`, `lower` or `upper` convert an identifier or string literal,
/// respecting acronym boundaries; `lower` and `upper` retain existing separators.
/// `type` produces a canonical, collision-safe name for one Rust type: it begins
/// with `type:`, which is deliberately illegal in Rust identifiers, and drops
/// formatting while preserving structure, so `& 'a mut Vec < Option < T > >`
/// becomes `"type:&'a mut Vec<Option<T>>"`.
///
/// # Examples
///
/// ```
/// # use these_macros_should_be_illegal::stringify_as;
/// assert_eq!(stringify_as!(camel; some_HTTP_server), "someHttpServer");
/// assert_eq!(stringify_as!(screaming_snake; SomeHTTPServer), "SOME_HTTP_SERVER");
/// assert_eq!(
///     stringify_as!(type; fn((u8, u16), *const [u8; 4]) -> bool),
///     "type:fn((u8,u16),*const[u8;4])->bool",
/// );
/// ```
#[proc_macro]
pub fn stringify_as(input: TokenStream) -> TokenStream {
    qol::stringify_as::stringify_as(input.into()).into()
}
