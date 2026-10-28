//! Experimental procedural macros for compact local DSLs, recursive token
//! rewriting, and transformations of macro structure.
//!
//! The main entry points are [`enum_fn`](macro@enum_fn) for per-variant methods,
//! [`discriminated_str`](macro@discriminated_str) for reversible string discriminants, [`strutuct`](macro@strutuct) for
//! nested algebraic declarations, and [`expand`](macro@expand) for applying recursive syntax
//! extensions to an out-of-line module. Every macro is independent; import only
//! the syntax used by the caller.
//!
//! ```
//! # use these_macros_should_be_illegal::enum_fn;
//!
//! #[enum_fn(code -> Option<usize>, missing = None)]
//! enum Status {
//!     Ready = Some(200),
//!     Missing,
//! }
//!
//! assert_eq!(Status::Ready.code(), Some(200));
//! assert_eq!(Status::Missing.code(), None);
//! ```
//!
//! The [crate repository](https://github.com/AlexanderGolys/these-macros-should-be-illegal)
//! contains the complete book and runnable examples.
//!
//! Macro configuration uses one argument grammar per macro family. Every macro
//! documents its grammar and defaults in its **Arguments** section. Attribute
//! macros take arguments directly in `#[name(...)]`; function-like macros accept
//! the tagged `@macro_args(...);` prefix, which [`macro_args`](macro@macro_args) can insert at item
//! position. Existing inline forms remain available. Parameters are unexpanded
//! tokens: a macro call can supply a value only where the receiving macro emits
//! that expression unchanged, rather than inspecting its name or literal value.

mod generate {
    pub(crate) mod callable;
    pub(crate) mod complete_ops;
    pub(crate) mod discriminated_str;
    pub(crate) mod enum_fn;
    pub(crate) mod forall;
    pub(crate) mod log;
    pub(crate) mod overload_op;
    pub(crate) mod strutuct;
}

mod helpers {
    pub(crate) mod arguments;
    pub(crate) mod fresh_name;
    pub(crate) mod preprocessing;
    pub(crate) mod ungroup;
}

mod meta {
    pub(crate) mod expand;
    pub(crate) mod forward_attributes;
    pub(crate) mod macro_args;
    pub(crate) mod make_fn;
    pub(crate) mod perm;
    pub(crate) mod reflect;
}

mod qol {
    pub(crate) mod qf;
    pub(crate) mod stringify_as;
}

mod sand {
    pub(crate) mod literally_literal_string;
    pub(crate) mod shared_match_arms;
}

use crate::generate::*;
use crate::helpers::arguments::input;
use crate::meta::*;
use crate::qol::*;
use crate::sand::*;
use proc_macro::TokenStream;

macro_rules! make_attr {
    ($(#[$attribute:meta])* $name:ident => $implementation:path) => {
        $(#[$attribute])*
        #[proc_macro_attribute]
        pub fn $name(arguments: TokenStream, item: TokenStream) -> TokenStream {
            $implementation(arguments.into(), item.into()).into()
        }
    };
        ($(#[$attribute:meta])* $name:ident) => {
        $(#[$attribute])*
        #[proc_macro_attribute]
        pub fn $name(arguments: TokenStream, item: TokenStream) -> TokenStream {
            $name::$name(arguments.into(), item.into()).into()
        }
    };
}

macro_rules! make_proc {
    ($(#[$attribute:meta])* $name:ident => $implementation:path) => {
        $(#[$attribute])*
        #[proc_macro]
        pub fn $name(input: TokenStream) -> TokenStream {
            $implementation(input.into()).into()
        }
    };
        ($(#[$attribute:meta])* $name:ident) => {
        $(#[$attribute])*
        #[proc_macro]
        pub fn $name(input: TokenStream) -> TokenStream {
            $name::$name(input.into()).into()
        }
    };
    ($(#[$attribute:meta])* $name:ident => no_arguments $implementation:path) => {
        $(#[$attribute])*
        #[proc_macro]
        pub fn $name(input: TokenStream) -> TokenStream {
            input::without_arguments(input.into(), $implementation).into()
        }
    };
}

make_attr! {
    /// Gives one trait method a structural alias that [`make_fn!`](macro@make_fn) calls through.
    ///
    /// No shared callable trait is imposed: the attribute only marks which method of
    /// a user-owned trait the generated same-name macro forwards its arguments to.
    ///
    /// # Arguments
    ///
    /// Write `#[callable(method)]` directly on a trait.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | `method` | Name of an existing trait method, including a raw identifier | Required |
    ///
    /// One trailing comma is accepted. The selected method must exist; the
    /// structural-call method name reserved by this macro must remain unused.
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
    callable
}

make_attr! {
    /// Adds the operators that follow from the ones a type already implements.
    ///
    /// `Sub` completes subtraction from addition and negation; `Neg = <scalar>`
    /// completes negation from multiplication by that scalar.
    ///
    /// # Arguments
    ///
    /// Write `#[complete_ops(Sub, Neg = expression)]` directly on a type declaration.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | `Sub` | Completes subtraction using addition and negation; takes no value | Not generated |
    /// | `Neg = expression` | Rust expression for the scalar used to implement negation | Not generated |
    ///
    /// Select at least one operator. Entries are comma-separated, a trailing comma
    /// is accepted, and each operator may occur once. Scalar expressions are emitted
    /// unchanged, so macro calls are allowed there. Negation is generated before
    /// subtraction when both are selected.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::ops::{Add, AddAssign, Neg};
    /// # use these_macros_should_be_illegal::complete_ops;
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
    complete_ops
}

make_attr! {
    /// Assigns every enum variant one unique string literal.
    ///
    /// The named method maps a value to its literal, and a same-name macro maps a
    /// literal and payload back to the variant constructor.
    ///
    /// # Arguments
    ///
    /// Write `#[discriminated_str(method)]` directly on an enum.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | `method` | Identifier naming the generated public string accessor | Required |
    ///
    /// One trailing comma is accepted. Variant string literals and payload fields
    /// belong to the enum input, rather than the attribute argument list.
    ///
    /// # Examples
    ///
    /// ```
    /// # use these_macros_should_be_illegal::discriminated_str;
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
    #[doc(alias = "disctr")]
    discriminated_str
}

make_attr! {
    /// Short alias of [`discriminated_str`](macro@discriminated_str).
    ///
    /// # Arguments
    ///
    /// Write `#[disctr(method)]` directly on an enum. `method` is the required
    /// generated accessor name; one trailing comma is accepted. Variant string
    /// literals and payloads use the same syntax as `discriminated_str`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use these_macros_should_be_illegal::disctr;
    /// #[disctr(name)]
    /// enum Token { End = "end" }
    /// assert_eq!(Token::End.name(), "end");
    /// ```
    disctr => discriminated_str::discriminated_str
}

make_attr! {
    /// Generates an enum method from per-variant expressions written beside the variants.
    ///
    /// Write `#[enum_fn(const name -> Type, missing = expression)]` on an enum.
    /// The method borrows `&self`, returns exactly the written type, and is
    /// runtime-callable by default. Add `const` after visibility to request a
    /// `const fn`.
    ///
    /// # Arguments
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | `name` | Generated method's identifier | Required |
    /// | `Type` | Exact Rust return type after `->` | Required |
    /// | `missing` | Expression used by the wildcard arm | `::core::panic!()` |
    ///
    /// The method is private unless a Rust visibility precedes its name, such as
    /// `pub(crate) const name -> Type`. One trailing comma is accepted.
    /// Attributes written before the signature inside the arguments apply to
    /// the generated method. For example,
    /// `#[enum_fn(#[doc = "Returns the code."] const code -> u8)]` documents it.
    /// Without a method doc attribute, only the generated method receives
    /// `#[allow(missing_docs)]`; documentation on the enum stays on the enum.
    ///
    /// Each variant's RHS may be any Rust expression or a lambda. Lambda
    /// arguments borrow one field each; their count must equal the field count.
    /// Named fields require parameters with the same names in declaration order;
    /// tuple parameters choose their names. Unit variants and empty products
    /// take zero-argument lambdas. A `const` method cannot use lambdas, and Rust
    /// checks that its expressions and fallback are const-compatible. There is
    /// no implicit conversion, cloning, `Option`, or `Some` wrapping.
    /// Fixed-result methods preserve elided return references such as `&str`.
    /// Lambda methods use an explicit generated borrow lifetime so a result can
    /// borrow from the variant's fields.
    ///
    /// Variants without values reach `_ => missing`. This wildcard is always
    /// emitted, so `non_exhaustive` works in either attribute order. Conditional
    /// compilation is preserved on generated code.
    ///
    /// # Examples
    ///
    /// ```
    /// # use these_macros_should_be_illegal::enum_fn;
    /// #[enum_fn(const code -> u8)]
    /// enum Status {
    ///     Ready = 1,
    ///     Dynamic(String) = 0,
    /// }
    /// const READY: u8 = Status::Ready.code();
    /// assert_eq!(READY, 1);
    /// assert_eq!(Status::Dynamic(String::new()).code(), 0);
    /// ```
    ///
    /// Optional results are explicit in both the type and values:
    ///
    /// ```
    /// # use these_macros_should_be_illegal::enum_fn;
    /// #[enum_fn(description -> Option<&str>, missing = None)]
    /// enum Message {
    ///     Fixed = Some("fixed"),
    ///     Named { text: String } = |text| Some(text.as_str()),
    ///     Missing,
    /// }
    /// assert_eq!(Message::Fixed.description(), Some("fixed"));
    /// assert_eq!(Message::Named { text: "hello".into() }.description(), Some("hello"));
    /// assert_eq!(Message::Missing.description(), None);
    /// ```
    ///
    /// Named-field parameters cannot rename their fields:
    ///
    /// ```compile_fail
    /// # use these_macros_should_be_illegal::enum_fn;
    /// #[enum_fn(code -> u8)]
    /// enum Value { Named { code: u8 } = |c| *c }
    /// ```
    ///
    /// Tuple fields require one parameter each:
    ///
    /// ```compile_fail
    /// # use these_macros_should_be_illegal::enum_fn;
    /// #[enum_fn(code -> u8)]
    /// enum Value { Pair(u8, u8) = |pair| pair.0 + pair.1 }
    /// ```
    ///
    /// Fixed results can call runtime functions without a lambda:
    ///
    /// ```
    /// # use these_macros_should_be_illegal::enum_fn;
    /// fn label(value: u8) -> String { value.to_string() }
    /// #[enum_fn(label -> String)]
    /// enum Value { Fixed = label(2) }
    /// assert_eq!(Value::Fixed.label(), "2");
    /// ```

    enum_fn
}

make_attr! {
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
    /// A member keeps out of the prefix with an inert `#[tmsbi(skip)]` marker,
    /// which skips every fragment, or `#[tmsbi(skip(fragments))]`, whose
    /// fragments are written like a prefix and skip the identical distributed
    /// ones. An attribute written as a bare path, like `#[doc]`, skips every
    /// distributed attribute of that path, and a group of named fields skips the
    /// shared fields by their names. Each stacked `forall` reads the same markers
    /// and the last one removes them, so a fragment no prefix distributes is
    /// ignored. A field named from its tuple position cannot skip the shared type.
    /// The marker lives under `tmsbi`, the namespace of this crate, and `forall`
    /// claims only the `tmsbi` markers opening with `skip`.
    ///
    /// A prefix forwarded by a declarative macro is read like one written
    /// directly: `#[forall($(#[$attribute])* $visibility $shared)]` takes its
    /// parts from `meta`, `vis` and `ty` fragments, including a `vis` fragment
    /// that matched nothing, and a field name may be forwarded as a `ty` fragment.
    ///
    /// # Arguments
    ///
    /// Write `#[forall(attributes visibility types)]` directly on a struct, union,
    /// or enum. The parts may appear in any order, with optional commas.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | `attributes` | Any number of outer `#[...]` attributes | No shared attributes |
    /// | `visibility` | One Rust visibility | Preserve each member's visibility |
    /// | `types` | Rust types, or one `{ name: Type, ... }` field group | Preserve existing fields |
    ///
    /// Supply at least one part. Braced structs and unions accept shared attributes
    /// and visibility. Tuple structs accept one shared type when their fields are
    /// bare names. Enums accept multiple shared types or one nonempty named-field
    /// group, and accept no visibility. Named and unnamed shared fields cannot be
    /// mixed. Trailing commas are accepted.
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
    /// #[forall(u16, String)]
    /// enum Reply {
    ///     Moved,
    ///     #[tmsbi(skip(String))]
    ///     Empty,
    ///     #[tmsbi(skip)]
    ///     Unknown,
    /// }
    ///
    /// let point = Point { x: 0.25, y: 0.75 };
    /// let response = Response::Error(404, String::from("missing"));
    ///
    /// assert_eq!(point.x + point.y, 1.0);
    /// assert!(matches!(response, Response::Error(404, _)));
    /// assert!(matches!(Stage::Lower, Stage::Lower));
    /// assert!(matches!(Reply::Empty(204), Reply::Empty(204)));
    /// assert!(matches!(Reply::Unknown, Reply::Unknown));
    /// ```
    forall
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
/// [`forall`](macro@forall)(macro@crate::forall) writes the fields this one reads.
///
/// # Arguments
///
/// The derive itself is written `#[derive(Log)]`. Configure its generated enum
/// with at most one `#[log(attributes visibility enum Name)]` helper attribute.
///
/// | Argument | Accepted value | Default |
/// | --- | --- | --- |
/// | `attributes` | Outer attributes for the generated enum | No explicit attributes |
/// | `visibility` | Rust visibility | The struct's visibility |
/// | `enum` | Optional keyword before the name | Omitted |
/// | `Name` | Identifier for the generated enum | `Log` followed by the struct's name |
///
/// The helper may be omitted, bare (`#[log]`), or empty (`#[log()]`). A trailing
/// comma after the name is accepted. Without an explicit `derive` attribute in
/// the helper, the enum derives `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, and
/// `Debug`. These are helper arguments; no [`macro_args`](macro@macro_args) wrapper is involved.
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
    log::log(item.into()).into()
}

make_attr! {
    /// Repeats an operator implemented for references across its owned forms.
    ///
    /// From `impl Op<&Rhs> for &Lhs` it generates `Op<&Rhs>` and `Op<Rhs>` for `Lhs`,
    /// and, when the result is `Lhs` itself, both `OpAssign` impls. Every generated
    /// impl delegates to the attributed one; nothing is cloned.
    ///
    /// # Arguments
    ///
    /// Takes no arguments: write `#[overload_op]` or `#[overload_op()]` directly
    /// on an operator implementation for references. The trait, operands, generics,
    /// and output type are read from that implementation. Any nonempty argument
    /// list is rejected.
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
    overload_op
}

make_proc! {
    /// Declares a family of nested types in place and hoists them into ordinary items.
    ///
    /// Braced bodies become structs or enums by shape and parenthesized bodies
    /// become tuple structs; every generated declaration is private unless a
    /// visibility says otherwise, and each gets a same-name constructor macro.
    /// [`emmun!`](macro@emmun) is an exact alias.
    ///
    /// # Arguments
    ///
    /// Supply root defaults through `#[macro_args(...)]` on an item-position
    /// invocation, or through `strutuct! { @macro_args(...); declaration }`.
    /// The same grammar is accepted by local `#[strutuct(...)]` configuration.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | Visibility | `pub`, restricted `pub(...)`, or explicit `priv` | Private |
    /// | `inclusions` / `no_inclusions` (`inc` / `no_inc`) | Bare flag; generate consuming enum inclusion functions | Disabled |
    /// | `product_variants` / `no_product_variants` (`prod` / `no_prod`) | Bare flag; pack a variant's fields into one product payload | Disabled |
    /// | `reverse_concat` / `no_reverse_concat` (`rev` / `no_rev`) | Bare flag; reverse generated name concatenation | Disabled |
    /// | `allow_missing_docs` / `no_allow_missing_docs` (`undoc` / `no_undoc`) | Bare flag; allow undocumented declarations and members throughout this branch | Disabled |
    ///
    /// Arguments are optional and comma-separated, with a trailing comma allowed.
    /// Full names and short aliases are interchangeable, including their `no_` forms.
    /// A flag enables its option; its `no_` form disables it. Both forms of the
    /// same option in one list count as duplicates, including mixed full and short names. Boolean assignments are rejected.
    /// Unknown or duplicate options in one list are rejected. Local configuration
    /// overrides inherited defaults for its branch; an explicitly written visibility
    /// applies at that declaration or field. The body is the nested declaration DSL,
    /// including any ordinary attributes to propagate.
    ///
    /// Documentation on declarations, variants, and fields is preserved as written;
    /// no placeholder descriptions are added. Ordinary `missing_docs` lint settings
    /// apply. Use `allow_missing_docs` (`undoc`) to emit `#[allow(missing_docs)]`
    /// throughout a generated branch, including hoisted nested declarations.
    /// `no_allow_missing_docs` (`no_undoc`) stops inheriting that option for a branch;
    /// ordinary Rust lint inheritance still applies within each declaration.
    ///
    /// Missing documentation remains an error when the caller denies it:
    ///
    /// ```rust,compile_fail
    /// #![deny(missing_docs)]
    /// //! A crate requiring documentation.
    /// use these_macros_should_be_illegal::strutuct;
    /// strutuct! { pub Undocumented { pub value: u8 } }
    /// fn main() {}
    /// ```
    ///
    /// Opt out for the complete generated family:
    ///
    /// ```rust
    /// #![deny(missing_docs)]
    /// //! A crate allowing one undocumented family.
    /// use these_macros_should_be_illegal::{macro_args, strutuct};
    /// #[macro_args(pub, undoc)]
    /// strutuct! { Undocumented { pub child: Child { pub value: u8 } } }
    /// fn main() {}
    /// ```
    ///
    /// Declaration-local attributes are separate from root parameters:
    ///
    /// | Attribute | Arguments and effect | Default |
    /// | --- | --- | --- |
    /// | `#[strutuct(...)]` | The flags and visibility above, inherited by this branch | Inherit enclosing configuration |
    /// | `#[derive(Trait, ...)]` | Comma-separated trait paths added to this declaration and its descendants | Inherit enclosing derives |
    /// | `#[underive(Trait, ...)]` | Comma-separated trait paths removed from this branch's inherited derives | Remove nothing |
    /// | `#[doc = "..."]` | Documentation for this declaration only | None |
    /// | `#[cfg(...)]`, `#[cfg_attr(...)]` | Ordinary Rust conditional attributes, also guarding constructor macros | Unconditional |
    /// | Other outer attributes | Forwarded unchanged according to ordinary attribute syntax | None |
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
    strutuct
}

make_proc! {
    /// An exact alias of [`strutuct!`](macro@strutuct), for declarations read as enums.
    ///
    /// # Arguments
    ///
    /// Accepts exactly the [`strutuct!`](macro@strutuct#arguments) argument grammar.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | Visibility | `pub`, restricted `pub(...)`, or explicit `priv` | Private |
    /// | `inclusions` / `no_inclusions` (`inc` / `no_inc`) | Bare flag | Disabled |
    /// | `product_variants` / `no_product_variants` (`prod` / `no_prod`) | Bare flag | Disabled |
    /// | `reverse_concat` / `no_reverse_concat` (`rev` / `no_rev`) | Bare flag | Disabled |
    /// | `allow_missing_docs` / `no_allow_missing_docs` (`undoc` / `no_undoc`) | Bare flag; allow undocumented declarations and members throughout this branch | Disabled |
    ///
    /// Use `#[macro_args(...)]` or the direct `@macro_args(...);` prefix. Local
    /// configuration is spelled `#[strutuct(...)]`, including when the invocation is
    /// named `emmun!`. Precedence, comma rules, and body syntax match `strutuct!`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use these_macros_should_be_illegal::emmun;
    /// emmun! { Choice { Yes, No } }
    /// assert!(matches!(Choice::Yes, Choice::Yes));
    /// ```
    emmun => strutuct::strutuct
}

// Low-level syntax extensions rewriting a whole token stream.

make_proc! {
    /// Rewrites literal strings in exactly the supplied token stream.
    ///
    /// An optional leading `exclude_macros = (names);` option keeps the invocations
    /// of the listed macros opaque, so the rewriting never descends into them.
    ///
    /// # Arguments
    ///
    /// Supply options through `#[macro_args(...)]` on an item-position invocation,
    /// or write `literally_literal_string! { @macro_args(...); body }` directly.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | `exclude_macros` (`exclude`) | `= (name, ...)`, a list of macro identifiers to keep opaque | Empty list |
    ///
    /// The option is optional and may occur once; mixing its full and short names
    /// also counts as a duplicate. Trailing commas are accepted both
    /// inside the list and after the option. Names match the final macro identifier
    /// exactly, including rawness; use `macro_rules` to exclude declarative macro
    /// bodies. The older `exclude_macros = (names); body` form is also accepted.
    /// The body is an arbitrary token stream and is rewritten recursively.
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
    #[doc(alias = "lls")]
    literally_literal_string
}

make_proc! {
    /// Short alias of [`literally_literal_string!`](macro@literally_literal_string).
    ///
    /// # Arguments
    ///
    /// Accepts `exclude_macros = (names)` (short name `exclude`), defaulting to an
    /// empty list. Supply it through `#[macro_args(...)]`, `@macro_args(...);`,
    /// or the inline `exclude_macros = (names);` form. The body is an arbitrary
    /// token stream, recursively rewritten with the same exclusions and comma
    /// rules as `literally_literal_string!`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use these_macros_should_be_illegal::lls;
    /// let owned: String = lls!(@@"owned");
    /// assert_eq!(owned, "owned");
    /// ```
    lls => literally_literal_string::literally_literal_string
}

make_proc! {
    /// Clones a shared match-arm RHS for alternatives separated by `||`.
    ///
    /// Parenthesize `(pattern if guard)` when one alternative has its own guard. An
    /// optional leading `exclude_macros = (names);` option keeps the invocations of
    /// the listed macros opaque, so the rewriting never descends into them.
    ///
    /// # Arguments
    ///
    /// Supply options through `#[macro_args(...)]` on an item-position invocation,
    /// or write `shared_match_arms! { @macro_args(...); body }` directly.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | `exclude_macros` (`exclude`) | `= (name, ...)`, a list of macro identifiers to keep opaque | Empty list |
    ///
    /// The option is optional and may occur once; mixing its full and short names
    /// also counts as a duplicate. Trailing commas are accepted in
    /// the list and after the option. Names match the final macro identifier exactly,
    /// including rawness; `macro_rules` excludes declarative macro bodies. The older
    /// `exclude_macros = (names); body` form remains accepted. The body is an
    /// arbitrary token stream, with nested groups rewritten recursively.
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
    #[doc(alias = "sma")]
    shared_match_arms
}

make_proc! {
    /// Short alias of [`shared_match_arms!`](macro@shared_match_arms).
    ///
    /// # Arguments
    ///
    /// Accepts `exclude_macros = (names)` (short name `exclude`), defaulting to an
    /// empty list. Supply it through `#[macro_args(...)]`, `@macro_args(...);`,
    /// or the inline `exclude_macros = (names);` form. The body is an arbitrary
    /// token stream, recursively rewritten with the same exclusions, guards,
    /// and comma rules as `shared_match_arms!`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use these_macros_should_be_illegal::sma;
    /// assert_eq!(sma!(match 2 { 1 || 2 => "small", _ => "other" }), "small");
    /// ```
    sma => shared_match_arms::shared_match_arms
}

// Transformers of other macros and of the token streams passed to them.

make_attr! {
    /// Supplies separate parameters to an item-position function-like macro invocation.
    ///
    /// The attribute prepends `@macro_args(parameters);` inside the invocation,
    /// preserving its path, delimiters, outer attributes and body tokens. The
    /// receiving macro parses these parameters with its own argument grammar.
    /// Repeated envelopes are rejected. Expression and type invocations can use the
    /// same prefix directly, since Rust does not allow this attribute there.
    ///
    /// The syntax-rewriting macros accept `exclude_macros = (names)`,
    /// [`strutuct!`](macro@strutuct) and [`emmun!`](macro@emmun) accept their root configuration
    /// options, [`stringify_as!`](macro@stringify_as) accepts its spelling option, and
    /// [`expand!`](macro@expand) accepts macro paths and shared preprocessing options.
    /// Other function-like macros in this crate accept only empty parameters.
    /// Existing inline argument forms remain supported.
    /// Place this attribute above [`forward_attributes`](macro@forward_attributes) when combining them.
    ///
    /// # Arguments
    ///
    /// Write `#[macro_args(parameters)]` directly on an item-position function-like
    /// macro invocation.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | `parameters` | Opaque tokens in the receiving macro's documented argument grammar | Empty token stream |
    ///
    /// This attribute forwards the tokens unchanged; validation and defaults belong
    /// to the receiving macro. It adds one `@macro_args(parameters);` prefix. A
    /// receiver in this crate rejects repeated prefixes. Keep this attribute above
    /// [`forward_attributes`](macro@forward_attributes) when combining them. For expression or type positions,
    /// write the prefix directly inside the receiving invocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use these_macros_should_be_illegal::{macro_args, strutuct, stringify_as};
    ///
    /// #[macro_args(product_variants)]
    /// strutuct! { Message Pair(u8, u8) }
    ///
    /// fn main() {
    ///     let message = Message::Pair((1, 2));
    ///     assert!(matches!(message, Message::Pair((1, 2))));
    ///     assert_eq!(stringify_as!(@macro_args(snake); SomeName), "some_name");
    /// }
    /// ```
    macro_args
}

make_proc! {
    /// Loads an out-of-line module and injects function-like macros around its body.
    ///
    /// The module file is found as for an ordinary `mod` declaration, or through a
    /// `#[path]` attribute, and may contain syntax that only the injected macros
    /// accept. Here it uses `@@"..."` owned string literals:
    ///
    /// # Arguments
    ///
    /// Write `expand!(macros, options; module)`, or supply the same arguments via
    /// `#[macro_args(macros, options)]` and leave only the module in the body.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | `macros` | One or more Rust macro paths, including qualified paths | Required |
    /// | `exclude_macros` (`exclude`) | `= (name, ...)`, opaque macro identifiers passed to each injected macro | Empty list |
    ///
    /// `exclude` is an alias for `exclude_macros`, with the same validation.
    /// Paths and options may be interspersed and are comma-separated; trailing
    /// commas are accepted. The exclusion option may occur once. Macro paths are
    /// nested in the written order, with the first outermost. The body must be one
    /// out-of-line module declaration; its visibility and attributes are preserved,
    /// and `#[path = "..."]` selects an explicit module file.
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
    expand
}

make_attr! {
    /// Moves the outer attributes of an item-position macro invocation into its input.
    ///
    /// The receiving function-like macro then decides what the attributes mean.
    ///
    /// # Arguments
    ///
    /// Takes no arguments: write `#[forward_attributes]` or
    /// `#[forward_attributes()]` directly on an item-position macro invocation.
    /// Nonempty arguments are rejected. The invocation's remaining outer attributes
    /// are forwarded into its body before a `;` separator. An existing
    /// `@macro_args(...);` parameter prefix remains first.
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
    forward_attributes
}

make_proc! {
    /// Creates a local value and a same-name macro providing function-like calls.
    ///
    /// The value's type must expose the method generated by [`callable`](macro@callable). The
    /// accepted syntax is `[mut] name = expression`.
    ///
    /// # Arguments
    ///
    /// Takes no separate parameters. An optional `@macro_args();` prefix is
    /// accepted; a nonempty parameter list is rejected.
    ///
    /// The body is `[mut] name = expression`, with one optional trailing semicolon.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | `mut` | Bare keyword allowing mutable access to the bound value | Immutable binding |
    /// | `name` | Identifier naming both the binding and the generated macro | Required |
    /// | `expression` | Any Rust expression, emitted unchanged | Required |
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
    make_fn => no_arguments meta::make_fn::make_fn
}

make_proc! {
    /// Permutes comma-separated token trees using standard cycle notation.
    ///
    /// Cycles compose from right to left and positions are one-based. Token trees
    /// beyond the largest mentioned position remain fixed. Spaces are required
    /// between positions because `14` is one integer token while `1 4` is two.
    ///
    /// ```rust,ignore
    /// perm! { ((1 4 3)), a, b, c, d, e }
    /// // expands to the token stream: c, b, d, a, e
    /// ```
    ///
    /// An empty cycle product demonstrates the identity in an ordinary expression
    /// context; nontrivial products deliberately return a raw comma-separated
    /// stream for structural consumers.
    ///
    /// # Arguments
    ///
    /// Takes no separate parameters. An optional `@macro_args();` prefix is
    /// accepted; a nonempty parameter list is rejected.
    ///
    /// The body is `(cycles), tree, ...`.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | `cycles` | Adjacent parenthesized cycles, each containing distinct positive integer positions | Empty product `()` is the identity |
    /// | `tree, ...` | Comma-separated individual token trees; group multi-token expressions | Empty stream when no positions are referenced |
    ///
    /// Each cycle is a parenthesized sequence of
    /// space-separated positive integer positions, such as `(1 4 3)`. Multiple
    /// cycles are adjacent inside the outer parentheses. An empty cycle product
    /// `()` is the identity. Cycles compose from right to left; zero positions,
    /// repeated positions within one cycle, and references beyond the supplied
    /// trees are rejected. Operands are individual token trees separated by commas,
    /// with a trailing comma accepted; group multi-token expressions as one tree.
    ///
    /// # Examples
    ///
    /// ```
    /// # use these_macros_should_be_illegal::perm;
    /// let fixed: &str = perm!((), "fixed");
    /// assert_eq!(fixed.len(), 5);
    /// ```
    perm => no_arguments meta::perm::perm
}

make_proc! {
    /// Reflects two macro invocation objects around an opaque token-stream body.
    ///
    /// `reflect!(first, second; body)` constructs
    /// `second! { first! { body } }`. Consequently `second` expands before
    /// `first`, reversing the expansion order of `first! { second! { body } }`.
    ///
    /// # Arguments
    ///
    /// Takes no separate parameters. An optional `@macro_args();` prefix is
    /// accepted; a nonempty parameter list is rejected.
    ///
    /// The body is `first, second; body`.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | `first` | Macro path, optionally qualified and preceded by outer attributes; becomes the inner invocation | Required |
    /// | `second` | Macro path, optionally qualified and preceded by outer attributes; becomes the outer invocation | Required |
    /// | `body` | Opaque tokens supplied to the inner invocation | Empty token stream |
    ///
    /// # Examples
    ///
    /// ```
    /// # use these_macros_should_be_illegal::reflect;
    /// macro_rules! add_one { ($value:expr) => { 1 + $value }; }
    /// macro_rules! double { ($value:expr) => { 2 * $value }; }
    /// assert_eq!(reflect!(add_one, double; 3), 8);
    /// ```
    reflect => no_arguments meta::reflect::reflect
}

// Remaining conveniences over ordinary Rust syntax.

make_proc! {
    /// Qualifies common unqualified standard-library types inside one Rust type.
    ///
    /// Nested common types are qualified recursively, while an already qualified
    /// path is preserved deliberately.
    ///
    /// # Arguments
    ///
    /// Takes no separate parameters. An optional `@macro_args();` prefix is
    /// accepted; a nonempty parameter list is rejected.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | Body | Exactly one Rust type, including generics, lifetimes, pointers, arrays, tuples, and function types | Required |
    ///
    /// Recognized unqualified names are
    /// `String`, `Box`, `Vec`, `HashMap`, `HashSet`, `BTreeMap`, `BTreeSet`, `Option`,
    /// and `Arc`. Their nested type arguments are visited recursively. There are no
    /// configuration defaults.
    ///
    /// # Examples
    ///
    /// ```
    /// # use these_macros_should_be_illegal::qf;
    /// let value: qf!(Option<Vec<String>>) = Some(vec![String::from("less punctuation")]);
    /// assert!(value.is_some());
    /// ```
    qf => no_arguments qol::qf::qf
}

make_proc! {
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
    /// # Arguments
    ///
    /// Write `stringify_as!(spelling; body)` or
    /// `stringify_as!(@macro_args(spelling); body)`. An item-position invocation may
    /// instead receive its spelling through `#[macro_args(spelling)]`.
    ///
    /// | Argument | Accepted value | Default |
    /// | --- | --- | --- |
    /// | `spelling` | One option from the table below | Required |
    /// | `body` | Identifier or nonempty string literal for case options; one Rust type for `type` | Required |
    ///
    /// | Option | Result | Body |
    /// | --- | --- | --- |
    /// | `camel` | `lowerCamelCase` | Identifier or nonempty string literal |
    /// | `pascal` | `UpperCamelCase` | Identifier or nonempty string literal |
    /// | `snake` | `snake_case` | Identifier or nonempty string literal |
    /// | `kebab` | `kebab-case` | Identifier or nonempty string literal |
    /// | `screaming_snake` | `SCREAMING_SNAKE_CASE` | Identifier or nonempty string literal |
    /// | `lower` | Unicode lowercase, preserving separators | Identifier or nonempty string literal |
    /// | `upper` | Unicode uppercase, preserving separators | Identifier or nonempty string literal |
    /// | `type` | Compact canonical spelling prefixed with `type:` | One Rust type |
    ///
    /// Exactly one spelling option is required, without a trailing comma. Raw
    /// identifiers contribute their name without `r#`. Macro calls standing for a
    /// name are rejected because the macro must inspect the name before expansion.
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
    stringify_as
}
