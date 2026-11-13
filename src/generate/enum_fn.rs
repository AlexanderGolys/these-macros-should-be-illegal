//! Local enum-syntax extension for methods generated as match expressions.
//!
//! The attribute reuses the discriminant position `Variant = ...` to hold the
//! value one method returns for that variant. Every such right-hand side is
//! removed from the enum and becomes one arm of a `match self` in an inherent
//! method; variants without one fall through to the `missing` expression.
//!
//! ```rust,ignore
//! #[enum_fn(pub const code -> u8, missing = 0)]
//! enum Status {
//!     Ready = 1,
//!     Busy(u32) = 2,
//!     Unknown,
//! }
//! assert_eq!(Status::Busy(7).code(), 2);
//! assert_eq!(Status::Unknown.code(), 0);
//! ```
//!
//! Expands to:
//!
//! ```rust,ignore
//! enum Status {
//!     Ready,
//!     Busy(u32),
//!     Unknown,
//! }
//! impl Status {
//!     #[allow(missing_docs)]
//!     pub const fn code(&self) -> u8 {
//!         match self {
//!             Self::Ready => 1,
//!             Self::Busy(..) => 2,
//!             #[allow(unreachable_patterns)]
//!             _ => 0,
//!         }
//!     }
//! }
//! ```
//!
//! A lambda right-hand side receives borrows of the variant's fields, and the
//! method gains a generated lifetime so the result may borrow from them:
//!
//! ```rust,ignore
//! #[enum_fn(label -> Option<&str>, missing = None)]
//! enum Message {
//!     Fixed = Some("fixed"),
//!     Text { body: String } = |body| Some(body.as_str()),
//!     Empty,
//! }
//! let text = Message::Text { body: "hello".into() };
//! assert_eq!(text.label(), Some("hello"));
//! assert_eq!(Message::Empty.label(), None);
//! ```
//!
//! Expands to:
//!
//! ```rust,ignore
//! enum Message {
//!     Fixed,
//!     Text { body: String },
//!     Empty,
//! }
//! impl Message {
//!     #[allow(missing_docs)]
//!     fn label<'__enum_fn_borrow>(&'__enum_fn_borrow self) -> Option<&'__enum_fn_borrow str> {
//!         match self {
//!             Self::Fixed => Some("fixed"),
//!             Self::Text { body: __enum_fn_field_0 } => {
//!                 (|body: &'__enum_fn_borrow String| Some(body.as_str()))(__enum_fn_field_0)
//!             }
//!             #[allow(unreachable_patterns)]
//!             _ => None,
//!         }
//!     }
//! }
//! ```

use proc_macro2::{Span, TokenStream};

use crate::helpers::{fresh_name, ungroup};
use quote::quote;
use syn::{
    Attribute, Error, Expr, ExprClosure, Fields, Ident, ItemEnum, Lifetime, Meta, Pat, PatType,
    Token, Type, Variant, Visibility,
    parse::{Parse, ParseStream},
    parse_quote_spanned, parse2,
    punctuated::Punctuated,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
};
/// Generates an enum method whose body is a match over its variants.
///
/// Parse failures of either the arguments or the item are returned as a
/// `compile_error!` invocation instead of the expansion, so the attribute
/// rejects anything but an enum:
///
/// ```rust,ignore
/// // error: expected `enum`
/// #[enum_fn(code -> u8)]
/// struct Status(u8);
/// ```
#[must_use]
pub fn enum_fn(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<Arguments>(arguments)
        .and_then(|arguments| parse2::<ItemEnum>(item).map(|item| (arguments, item)))
        .and_then(|(arguments, item)| expand(arguments, item));

    result.unwrap_or_else(Error::into_compile_error)
}
/// Arguments of one `#[enum_fn(...)]` attribute: the generated method's
/// signature followed by optional comma-separated options.
///
/// The signature is written as
/// `#[attributes]* visibility? const? name -> ReturnType`, and the only option
/// is `missing = expression`, the result for variants without a right-hand
/// side. A single trailing comma is accepted.
///
/// ```rust,ignore
/// // Private runtime method; the default `missing` panics.
/// #[enum_fn(code -> u8)]
/// enum Bit {
///     Zero = 0,
///     One = 1,
/// }
/// assert_eq!(Bit::One.code(), 1);
///
/// // Restricted visibility, `const`, an explicit fallback, and a trailing comma.
/// #[enum_fn(pub(crate) const weight -> u8, missing = 0,)]
/// enum Coin {
///     Gold = 10,
///     Copper,
/// }
/// const COPPER: u8 = Coin::Copper.weight();
/// assert_eq!(COPPER, 0);
///
/// // Attributes before the signature apply to the generated method.
/// #[enum_fn(#[doc = "Human-readable name."] #[inline] pub name -> Option<&str>, missing = None)]
/// enum Planet {
///     Earth = Some("Earth"),
///     Unnamed,
/// }
/// assert_eq!(Planet::Unnamed.name(), None);
/// ```
struct Arguments {
    method_attributes: Vec<Attribute>,
    method_visibility: Visibility,
    is_const: Option<Token![const]>,
    method_name: Ident,
    return_type: Box<Type>,
    missing: Expr,
}

/// Parses the `enum_fn` attribute's arguments.
impl Parse for Arguments {
    /// Parses method attributes, visibility, optional constness, method name,
    /// return type, and the options that follow.
    ///
    /// Any option other than `missing` is rejected:
    ///
    /// ```rust,ignore
    /// // error: unknown option; expected `missing = expression`
    /// #[enum_fn(code -> u8, fallback = 0)]
    /// enum Bit {
    ///     One = 1,
    /// }
    /// ```
    ///
    /// So is a second `missing`:
    ///
    /// ```rust,ignore
    /// // error: duplicate `missing` option
    /// #[enum_fn(code -> u8, missing = 1, missing = 2)]
    /// enum Bit {
    ///     One = 1,
    /// }
    /// ```
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method_attributes = Attribute::parse_outer(input)?;
        let visibility = input.parse()?;
        let is_const = input.parse()?;
        let method = input.parse()?;
        input.parse::<Token![->]>()?;
        let output = input.parse()?;
        let mut missing = None;
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let option: Ident = input.parse()?;
            if option != "missing" {
                return Err(Error::new_spanned(
                    option,
                    "unknown option; expected `missing = expression`",
                ));
            }
            if missing.is_some() {
                return Err(Error::new_spanned(option, "duplicate `missing` option"));
            }
            input.parse::<Token![=]>()?;
            missing = Some(input.parse()?);
        }
        let missing = missing.unwrap_or_else(|| syn::parse_quote!(::core::panic!()));

        Ok(Self {
            method_attributes,
            method_visibility: visibility,
            is_const,
            method_name: method,
            return_type: output,
            missing,
        })
    }
}

/// Right-hand side written after `=` on one variant, which decides what its
/// match arm evaluates.
///
/// An expression ignores the variant's fields; a lambda receives a borrow of
/// each of them.
///
/// ```rust,ignore
/// #[enum_fn(area -> u32)]
/// enum Shape {
///     // Expression: the arm is `Self::Dot => 0`.
///     Dot = 0,
///     // Expression: the arm is `Self::Line(..) => 0`.
///     Line(u32) = 0,
///     // Call: the lambda receives `&u32, &u32`.
///     Rectangle(u32, u32) = |width, height| width * height,
///     // Call: the lambda receives `&u32`.
///     Square { side: u32 } = |side| side * side,
/// }
/// assert_eq!(Shape::Line(5).area(), 0);
/// assert_eq!(Shape::Rectangle(2, 3).area(), 6);
/// assert_eq!(Shape::Square { side: 4 }.area(), 16);
/// ```
enum VariantOutput {
    /// Evaluate an ordinary Rust expression that ignores the variant's fields.
    Expression(Box<Expr>),
    /// Invoke a closure with a borrow of every field, in declaration order.
    Call(Box<ExprClosure>),
}

/// Removes method-arm expressions and emits the accessor implementation.
///
/// Only the enum's conditional attributes are copied onto the `impl`, so the
/// method disappears together with an excluded enum. A lambda anywhere in the
/// enum switches the whole method to an explicit receiver lifetime.
///
/// ```rust,ignore
/// #[enum_fn(code -> u8)]
/// #[cfg(unix)]
/// #[derive(Debug)]
/// enum Status {
///     Ready = 1,
///     Busy(u32) = |load| *load as u8,
/// }
/// ```
///
/// Expands to:
///
/// ```rust,ignore
/// #[cfg(unix)]
/// #[derive(Debug)]
/// enum Status {
///     Ready,
///     Busy(u32),
/// }
/// #[cfg(unix)]
/// impl Status {
///     #[allow(missing_docs)]
///     fn code<'__enum_fn_borrow>(&'__enum_fn_borrow self) -> u8 {
///         match self {
///             Self::Ready => 1,
///             Self::Busy(__enum_fn_field_0) => {
///                 (|load: &'__enum_fn_borrow u32| *load as u8)(__enum_fn_field_0)
///             }
///             #[allow(unreachable_patterns)]
///             _ => ::core::panic!(),
///         }
///     }
/// }
/// ```
fn expand(arguments: Arguments, mut item: ItemEnum) -> syn::Result<TokenStream> {
    let Arguments {
        method_attributes,
        method_visibility: visibility,
        is_const,
        method_name: method,
        return_type: output,
        missing,
    } = arguments;
    let mut outputs = Vec::with_capacity(item.variants.len());
    let mut has_lambda = false;
    for variant in &mut item.variants {
        let discriminant = variant
            .discriminant
            .take()
            .map(|(eq, expression)| (eq, ungroup::expr(&expression).clone()));

        let output = match discriminant {
            Some((_, Expr::Closure(closure))) => {
                validate_closure(&variant.fields, &closure)?;
                has_lambda = true;
                Some(VariantOutput::Call(Box::new(closure)))
            }
            Some((_, expression)) => Some(VariantOutput::Expression(Box::new(expression))),
            None => None,
        };
        outputs.push(output);
    }

    let ident = &item.ident;
    let (impl_generics, type_generics, where_clause) = item.generics.split_for_impl();
    if is_const.is_some() && has_lambda {
        return Err(Error::new_spanned(
            &method,
            "`const enum_fn` cannot use lambda expressions",
        ));
    }
    let constness = is_const.is_some().then(|| quote!(const));
    let (return_type, method_lifetime, receiver_lifetime, borrow) = if has_lambda {
        let written_names = fresh_name::written_identifiers(quote!(#item #output));
        let borrow_name = fresh_name::fresh_name("__enum_fn_borrow", &written_names);
        let borrow = Lifetime::new(&format!("'{borrow_name}"), Span::mixed_site());
        let mut return_type = output;
        BorrowedReturn { borrow: &borrow }.visit_type_mut(&mut return_type);
        (
            return_type,
            quote!(<#borrow>),
            quote!(#borrow),
            Some(borrow),
        )
    } else {
        (output, quote!(), quote!(), None)
    };
    let arms = item
        .variants
        .iter()
        .zip(&outputs)
        .filter_map(|(variant, output)| {
            output
                .as_ref()
                .map(|output| accessor_arm(variant, output, borrow.as_ref()))
        });
    let method_attributes = if method_attributes.iter().any(|attribute| {
        attribute.path().is_ident("doc") && matches!(&attribute.meta, Meta::NameValue(_))
    }) {
        quote!(#(#method_attributes)*)
    } else {
        quote!(#[allow(missing_docs)] #(#method_attributes)*)
    };
    let method = quote! {
        #method_attributes
        #visibility #constness fn #method #method_lifetime (& #receiver_lifetime self) -> #return_type {
            match self {
                #(#arms,)*
                #[allow(unreachable_patterns)]
                _ => #missing,
            }
        }
    };

    let impl_attrs = conditional_attrs(&item.attrs);
    Ok(quote! {
        #item
        #(#impl_attrs)*
        impl #impl_generics #ident #type_generics #where_clause {
            #method
        }
    })
}

/// Checks that a mapping lambda describes exactly the variant's product.
///
/// The lambda takes one parameter per field. Tuple fields may be bound under
/// any pattern, while named fields must be bound by their own names in
/// declaration order, optionally with a type annotation. Accepted lambdas are
/// shown on [`closure_arm`]; each of the following is rejected.
///
/// A tuple variant needs one parameter per field:
///
/// ```rust,ignore
/// // error: expected 2 lambda argument(s), one per variant field; found 1
/// #[enum_fn(sum -> u8)]
/// enum Value {
///     Pair(u8, u8) = |pair| pair.0,
/// }
/// ```
///
/// A named field cannot be renamed:
///
/// ```rust,ignore
/// // error: expected lambda argument `code` matching the named field in declaration order
/// #[enum_fn(code -> u8)]
/// enum Value {
///     Named { code: u8 } = |c| *c,
/// }
/// ```
///
/// Named fields are bound in declaration order:
///
/// ```rust,ignore
/// // error: expected lambda argument `x` matching the named field in declaration order
/// #[enum_fn(first -> u8)]
/// enum Value {
///     Point { x: u8, y: u8 } = |y, x| *x,
/// }
/// ```
fn validate_closure(fields: &Fields, closure: &ExprClosure) -> syn::Result<()> {
    if closure.inputs.len() != fields.len() {
        return Err(Error::new_spanned(
            &closure.inputs,
            format!(
                "expected {} lambda argument(s), one per variant field; found {}",
                fields.len(),
                closure.inputs.len()
            ),
        ));
    }
    if let Fields::Named(fields) = fields {
        for (field, input) in fields.named.iter().zip(&closure.inputs) {
            let pattern = match input {
                Pat::Type(typed) => &*typed.pat,
                pattern => pattern,
            };
            let name = field.ident.as_ref().expect("named field has an identifier");
            if !matches!(pattern, Pat::Ident(binding) if binding.ident == *name && binding.subpat.is_none())
            {
                return Err(Error::new_spanned(
                    input,
                    format!(
                        "expected lambda argument `{name}` matching the named field in declaration order"
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Builds one match arm from its variant and selected operation.
///
/// The variant's conditional attributes guard the arm, so an arm never refers
/// to a variant that was compiled out.
///
/// ```rust,ignore
/// #[enum_fn(code -> u8)]
/// enum Status {
///     Ready = 1,
///     #[cfg(test)]
///     Mock = 0,
/// }
/// assert_eq!(Status::Ready.code(), 1);
/// ```
///
/// Expands to:
///
/// ```rust,ignore
/// enum Status {
///     Ready,
///     #[cfg(test)]
///     Mock,
/// }
/// impl Status {
///     #[allow(missing_docs)]
///     fn code(&self) -> u8 {
///         match self {
///             Self::Ready => 1,
///             #[cfg(test)]
///             Self::Mock => 0,
///             #[allow(unreachable_patterns)]
///             _ => ::core::panic!(),
///         }
///     }
/// }
/// assert_eq!(Status::Ready.code(), 1);
/// ```
fn accessor_arm(
    variant: &Variant,
    output: &VariantOutput,
    borrow: Option<&Lifetime>,
) -> TokenStream {
    let attrs = conditional_attrs(&variant.attrs);
    let (pattern, expression) = match output {
        VariantOutput::Expression(value) => (
            variant_pattern(&variant.ident, &variant.fields),
            quote!(#value),
        ),
        VariantOutput::Call(closure) => {
            let borrow = borrow.expect("lambda outputs have a generated borrow lifetime");
            let (pattern, value) = closure_arm(&variant.ident, &variant.fields, closure, borrow);
            (pattern, value)
        }
    };

    quote!(#(#attrs)* #pattern => #expression)
}

/// Lowers a mapping closure with hygienic bindings for every variant field.
///
/// Returns the arm's pattern and the immediately invoked closure separately.
/// The bindings use mixed-site hygiene, so they never capture or shadow names
/// used inside the closure body.
///
/// ```rust,ignore
/// #[enum_fn(size -> usize)]
/// enum Item {
///     Pair(u8, u8) = |x, y| usize::from(*x + *y),
///     Text { body: String } = |body| body.len(),
///     Unit = || 1,
/// }
/// assert_eq!(Item::Pair(1, 2).size(), 3);
/// assert_eq!(Item::Text { body: "four".into() }.size(), 4);
/// assert_eq!(Item::Unit.size(), 1);
/// ```
///
/// Expands to:
///
/// ```rust,ignore
/// enum Item {
///     Pair(u8, u8),
///     Text { body: String },
///     Unit,
/// }
/// impl Item {
///     #[allow(missing_docs)]
///     fn size<'__enum_fn_borrow>(&'__enum_fn_borrow self) -> usize {
///         match self {
///             Self::Pair(__enum_fn_field_0, __enum_fn_field_1) => {
///                 (|x: &'__enum_fn_borrow u8, y: &'__enum_fn_borrow u8| usize::from(*x + *y))(
///                     __enum_fn_field_0,
///                     __enum_fn_field_1,
///                 )
///             }
///             Self::Text { body: __enum_fn_field_0 } => {
///                 (|body: &'__enum_fn_borrow String| body.len())(__enum_fn_field_0)
///             }
///             Self::Unit => (|| 1)(),
///             #[allow(unreachable_patterns)]
///             _ => ::core::panic!(),
///         }
///     }
/// }
/// ```
fn closure_arm(
    variant: &Ident,
    fields: &Fields,
    closure: &ExprClosure,
    borrow: &Lifetime,
) -> (TokenStream, TokenStream) {
    let bindings: Vec<_> = (0..fields.iter().count())
        .map(|index| Ident::new(&format!("__enum_fn_field_{index}"), Span::mixed_site()))
        .collect();
    let pattern = match fields {
        Fields::Unit => quote!(Self::#variant),
        Fields::Unnamed(_) => quote!(Self::#variant(#(#bindings),*)),
        Fields::Named(fields) => {
            let entries = fields.named.iter().zip(&bindings).map(|(field, binding)| {
                let name = field
                    .ident
                    .as_ref()
                    .expect("fields in a named variant always have identifiers");
                quote!(#name: #binding)
            });
            quote!(Self::#variant { #(#entries),* })
        }
    };
    let closure = typed_closure(fields, closure, borrow);
    let value = quote!((#closure)(#(#bindings),*));
    (pattern, value)
}

/// Supplies field-reference types for closure inputs that the user left untyped.
///
/// Inputs that already carry a type annotation are kept as written, so
/// `|count, name: &String|` on a `Tagged(u8, String)` variant becomes
/// `|count: &'__enum_fn_borrow u8, name: &String|`.
///
/// ```rust,ignore
/// #[enum_fn(describe -> String)]
/// enum Entry {
///     Tagged(u8, String) = |count, name: &String| format!("{name} x{count}"),
/// }
/// assert_eq!(Entry::Tagged(2, "apple".into()).describe(), "apple x2");
/// ```
fn typed_closure(fields: &Fields, closure: &ExprClosure, borrow: &Lifetime) -> ExprClosure {
    let mut closure = closure.clone();
    for (input, field) in closure.inputs.iter_mut().zip(fields) {
        if matches!(input, Pat::Type(_)) {
            continue;
        }

        let pattern = input.clone();
        let ty = &field.ty;
        let typed: PatType = parse_quote_spanned!(pattern.span()=> #pattern: &#borrow #ty);
        *input = Pat::Type(typed);
    }
    closure
}

/// Builds a non-binding match pattern for a variant.
///
/// - `Ready` becomes `Self::Ready`.
/// - `Busy(u32)` becomes `Self::Busy(..)`.
/// - `Text { body: String }` becomes `Self::Text { .. }`.
///
/// ```rust,ignore
/// #[enum_fn(kind -> &str)]
/// enum Event {
///     Ready = "ready",
///     Busy(u32) = "busy",
///     Text { body: String } = "text",
/// }
/// assert_eq!(Event::Busy(3).kind(), "busy");
/// assert_eq!(Event::Text { body: String::new() }.kind(), "text");
/// ```
fn variant_pattern(ident: &Ident, fields: &Fields) -> TokenStream {
    match fields {
        Fields::Unit => quote!(Self::#ident),
        Fields::Unnamed(_) => quote!(Self::#ident(..)),
        Fields::Named(_) => quote!(Self::#ident { .. }),
    }
}

/// Names elided return-reference lifetimes consistently with the receiver.
///
/// Elided and `'_` references receive the generated borrow lifetime, while
/// named lifetimes and references inside function-pointer or `Fn(...)`
/// signatures, which have their own elision scope, are left untouched:
///
/// - `&str` becomes `&'__enum_fn_borrow str`.
/// - `Option<&'_ [u8]>` becomes `Option<&'__enum_fn_borrow [u8]>`.
/// - `&'static str`, `fn(&str) -> &str` and `Box<dyn Fn(&u8) -> &u8>` stay as written.
///
/// ```rust,ignore
/// #[enum_fn(bytes -> Option<&'_ [u8]>, missing = None)]
/// enum Payload {
///     Raw(Vec<u8>) = |data| Some(data.as_slice()),
///     Empty,
/// }
/// assert_eq!(Payload::Raw(vec![1, 2]).bytes(), Some(&[1, 2][..]));
///
/// #[enum_fn(trim -> fn(&str) -> &str)]
/// enum Trim {
///     Start(bool) = |_| str::trim_start,
///     End = str::trim_end,
/// }
/// assert_eq!((Trim::End.trim())("  x  "), "  x");
/// ```
struct BorrowedReturn<'a> {
    /// Lifetime of the generated shared receiver.
    borrow: &'a Lifetime,
}

/// Rewrites reference lifetimes while skipping nested elision scopes.
impl VisitMut for BorrowedReturn<'_> {
    /// Names an elided or `'_` reference lifetime, then visits the referent.
    fn visit_type_reference_mut(&mut self, reference: &mut syn::TypeReference) {
        if reference.lifetime.is_none()
            || reference
                .lifetime
                .as_ref()
                .is_some_and(|lifetime| lifetime.ident == "_")
        {
            reference.lifetime = Some(self.borrow.clone());
        }
        visit_mut::visit_type_reference_mut(self, reference);
    }

    /// Leaves `fn(&T) -> &U` alone, since its references elide among themselves.
    fn visit_type_fn_ptr_mut(&mut self, _: &mut syn::TypeFnPtr) {}

    /// Leaves `Fn(&T) -> &U` alone, since its references elide among themselves.
    fn visit_parenthesized_generic_arguments_mut(
        &mut self,
        _: &mut syn::ParenthesizedGenericArguments,
    ) {
    }
}

/// Retains only conditional compilation, including nested `cfg_attr` conditions.
///
/// - `#[cfg(test)] #[derive(Debug)] #[doc = "..."]` keeps `#[cfg(test)]`.
/// - `#[cfg_attr(unix, cfg(test), derive(Debug))] #[repr(u8)]` keeps
///   `#[cfg_attr(unix, cfg(test))]`.
///
/// ```rust,ignore
/// #[enum_fn(code -> u8, missing = 0)]
/// enum Flag {
///     #[cfg_attr(unix, doc = "Documented only on Unix.")]
///     Shown = 1,
///     #[cfg_attr(not(test), cfg(test))]
///     Hidden = 2,
/// }
/// assert_eq!(Flag::Shown.code(), 1);
/// ```
fn conditional_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter_map(|attribute| {
            let meta = conditional_meta(&attribute.meta)?;
            Some(parse_quote_spanned!(attribute.span()=> #[#meta]))
        })
        .collect()
}

/// Removes variant-only metadata from a conditional attribute before forwarding it.
///
/// Returns `None` when nothing conditional remains to forward:
///
/// - `cfg(test)` stays `cfg(test)`.
/// - `cfg_attr(unix, derive(Debug), cfg(test))` becomes `cfg_attr(unix, cfg(test))`.
/// - `cfg_attr(unix, cfg_attr(test, cfg(debug_assertions)), doc = "...")` becomes
///   `cfg_attr(unix, cfg_attr(test, cfg(debug_assertions)))`.
/// - `cfg_attr(unix, derive(Debug))` and `derive(Debug)` are dropped.
///
/// ```rust,ignore
/// #[enum_fn(code -> u8, missing = 0)]
/// enum Mode {
///     #[cfg_attr(unix, cfg_attr(not(test), cfg(test)), doc = "Unix-only docs.")]
///     Debug = 1,
///     Release = 2,
/// }
/// assert_eq!(Mode::Release.code(), 2);
/// ```
fn conditional_meta(meta: &Meta) -> Option<Meta> {
    if meta.path().is_ident("cfg") {
        return Some(meta.clone());
    }
    let Meta::List(list) = meta else {
        return None;
    };
    if !list.path.is_ident("cfg_attr") {
        return None;
    }
    let arguments = list
        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .ok()?;
    let mut arguments = arguments.iter();
    let condition = arguments.next()?;
    let attributes: Vec<_> = arguments.filter_map(conditional_meta).collect();
    if attributes.is_empty() {
        return None;
    }
    let mut filtered = list.clone();
    filtered.tokens = quote!(#condition, #(#attributes),*);
    Some(Meta::List(filtered))
}

/// Parser and diagnostic tests that do not require a downstream crate.
#[cfg(test)]
mod tests {
    use quote::quote;

    use super::enum_fn;

    /// Missing values are arbitrary expressions, with one documented option.
    #[test]
    fn validates_named_options() {
        let item = quote!(
            enum Value {
                Missing,
            }
        );
        for arguments in [
            quote!(value -> usize),
            quote!(const value -> usize),
            quote!(value -> Option<usize>, missing = None),
            quote!(value -> usize, missing = 42,),
            quote!(pub value -> usize, missing = panic!("missing")),
            quote!(value -> usize, missing = { let base = 3; base * 2 }),
        ] {
            let output = enum_fn(arguments, item.clone()).to_string();
            assert!(!output.contains("compile_error"), "{output}");
        }
        for (arguments, message) in [
            (quote!(value -> usize, fallback = 0), "unknown option"),
            (
                quote!(value -> usize, missing = 1, missing = 2),
                "duplicate `missing` option",
            ),
            (quote!(value -> usize = panic), "expected `,`"),
        ] {
            let output = enum_fn(arguments, item.clone()).to_string();
            assert!(output.contains(message), "{output}");
        }
    }

    /// Constness is an explicit method modifier, and const accessors reject lambdas.
    #[test]
    fn validates_explicit_constness() {
        let item = quote!(
            enum Value {
                Present = 1,
            }
        );
        let constant = enum_fn(quote!(const value -> usize), item.clone()).to_string();
        let runtime = enum_fn(quote!(value -> usize), item).to_string();

        assert!(constant.contains("const fn value"));
        assert!(runtime.contains("] fn value"));

        let lambda = enum_fn(
            quote!(const value -> usize),
            quote!(
                enum Value {
                    Dynamic(usize) = |value| *value,
                }
            ),
        )
        .to_string();
        assert!(lambda.contains("const enum_fn` cannot use lambda"));
    }

    /// Fixed results preserve elided lifetimes and accept method-level attributes.
    #[test]
    fn emits_only_requested_method_documentation() {
        let item = quote!(
            #[doc = "Enum documentation"]
            enum Value {
                Present = "value",
            }
        );
        let documented = enum_fn(
            quote!(#[doc = "Accessor documentation"] #[inline] value -> &str),
            item.clone(),
        )
        .to_string();

        assert!(documented.contains("Accessor documentation"));
        assert!(documented.contains("inline"));
        assert!(documented.contains("fn value (& self) -> & str"));
        assert!(!documented.contains("__enum_fn_borrow"));
        assert!(!documented.contains("Returns this variant"));
        assert!(documented.contains("Enum documentation"));

        let undocumented = enum_fn(quote!(value -> &str), item).to_string();
        assert!(undocumented.contains("allow (missing_docs)"));
        assert!(!undocumented.contains("Returns this variant"));

        let borrowed = enum_fn(
            quote!(value -> &str),
            quote!(
                enum Borrowed {
                    Text(String) = |text| text.as_str(),
                }
            ),
        )
        .to_string();
        assert!(borrowed.contains("__enum_fn_borrow"));
    }

    /// Lambda arguments describe the variant fields without renaming named fields.
    #[test]
    fn validates_lambda_signatures() {
        for item in [
            quote!(
                enum Value {
                    Named { code: u8 } = |code| *code,
                }
            ),
            quote!(
                enum Value {
                    Named { code: u8 } = |code: &u8| *code,
                }
            ),
            quote!(
                enum Value {
                    Tuple(u8, u8) = |x, y| *x + *y,
                }
            ),
            quote!(
                enum Value {
                    Empty {} = || 1,
                }
            ),
            quote!(
                enum Value {
                    Empty() = || 1,
                }
            ),
            quote!(
                enum Value {
                    Unit = || 1,
                }
            ),
        ] {
            let output = enum_fn(quote!(value -> u8), item).to_string();
            assert!(!output.contains("compile_error"), "{output}");
        }
        for (item, message) in [
            (
                quote!(
                    enum Value {
                        Named { code: u8 } = |c| *c,
                    }
                ),
                "expected lambda argument `code`",
            ),
            (
                quote!(
                    enum Value {
                        Named { code: u8 } = |_| 1,
                    }
                ),
                "expected lambda argument `code`",
            ),
            (
                quote!(
                    enum Value {
                        Named { x: u8, y: u8 } = |y, x| *x + *y,
                    }
                ),
                "expected lambda argument `x`",
            ),
            (
                quote!(
                    enum Value {
                        Tuple(u8, u8) = |x| x.0 + x.1,
                    }
                ),
                "expected 2 lambda argument(s)",
            ),
            (
                quote!(
                    enum Value {
                        Unit = |x| x,
                    }
                ),
                "expected 0 lambda argument(s)",
            ),
        ] {
            let output = enum_fn(quote!(value -> u8), item).to_string();
            assert!(output.contains(message), "{output}");
        }
    }

    /// The attribute is deliberately restricted to an enum item.
    #[test]
    fn rejects_a_non_enum_item() {
        let output = enum_fn(
            quote!(value -> usize),
            quote! {
                struct Value(usize);
            },
        )
        .to_string();

        assert!(output.contains("expected `enum`"));
    }

    /// The written visibility is applied to the method, which is private otherwise.
    #[test]
    fn applies_the_method_visibility() {
        let item = quote! {
            enum Value {
                Present = 1,
            }
        };
        let public = enum_fn(quote!(pub(crate) value -> usize), item.clone()).to_string();
        let const_public =
            enum_fn(quote!(pub(crate) const value -> usize), item.clone()).to_string();
        let constant = enum_fn(quote!(const value -> usize), item.clone()).to_string();
        let private = enum_fn(quote!(value -> usize), item).to_string();

        assert!(public.contains("pub (crate) fn value"));
        assert!(const_public.contains("pub (crate) const fn value"));
        assert!(constant.contains("const fn value"));
        assert!(private.contains("] fn value"));
    }
}

/// Generates a constant successor method using the enum accessor machinery.
pub fn enum_cycle(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = (|| -> syn::Result<TokenStream> {
        let original = parse2::<ItemEnum>(item)?;
        let mut mapped = original.clone();
        for (index, variant) in mapped.variants.iter_mut().enumerate() {
            if !matches!(variant.fields, Fields::Unit) {
                return Err(Error::new_spanned(
                    &variant.fields,
                    "enum_cycle requires unit variants",
                ));
            }
            let successors = original
                .variants
                .iter()
                .cycle()
                .skip(index + 1)
                .take(original.variants.len())
                .map(|candidate| {
                    let attributes = conditional_attrs(&candidate.attrs);
                    let name = &candidate.ident;
                    quote!(#(#attributes)* { return Self::#name; })
                });
            variant.discriminant = Some((
                Default::default(),
                syn::parse_quote!({
                    #[allow(unreachable_code)]
                    { #(#successors)* ::core::panic!("no enabled successor") }
                }),
            ));
        }
        let arguments = parse2::<CycleArguments>(arguments)?;
        let CycleArguments {
            attributes,
            visibility,
            name,
        } = arguments;
        let expanded = enum_fn(
            quote!(#(#attributes)* #visibility const #name -> Self),
            quote!(#mapped),
        );
        let mut file = parse2::<syn::File>(expanded)?;
        file.items[0] = syn::Item::Enum(original);
        Ok(quote!(#file))
    })();
    result.unwrap_or_else(Error::into_compile_error)
}

struct CycleArguments {
    attributes: Vec<Attribute>,
    visibility: Visibility,
    name: Ident,
}

impl Parse for CycleArguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let arguments = Self {
            attributes: input.call(Attribute::parse_outer)?,
            visibility: input.parse()?,
            name: input.parse()?,
        };
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        Ok(arguments)
    }
}

#[cfg(test)]
mod cycle_tests {
    use super::enum_cycle;
    use quote::quote;

    #[test]
    fn rejects_payload_variants() {
        for item in [
            quote!(
                enum Invalid {
                    Tuple(u8),
                }
            ),
            quote!(
                enum Invalid {
                    Named { value: u8 },
                }
            ),
            quote!(
                enum Invalid {
                    EmptyTuple(),
                }
            ),
            quote!(
                enum Invalid {
                    EmptyStruct {},
                }
            ),
        ] {
            let output = enum_cycle(quote!(next), item).to_string();
            assert!(
                output.contains("enum_cycle requires unit variants"),
                "{output}"
            );
        }
    }
}
