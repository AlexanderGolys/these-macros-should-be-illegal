//! Local enum-syntax extension for methods generated as match expressions.

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

/// Parsed accessor name and requested output representation.
struct Arguments {
    /// Attributes placed before the generated method signature.
    method_attributes: Vec<Attribute>,
    /// Visibility of the generated method.
    method_visibility: Visibility,
    /// Whether the generated method is explicitly const.
    is_const: bool,
    /// Name of the accessor method generated on the enum.
    method_name: Ident,
    /// Exact return type declared for the generated method.
    return_type: Type,
    /// Requested behavior for variants without an arm expression.
    missing: Expr,
}

/// Parses the `enum_fn` attribute's arguments.
impl Parse for Arguments {
    /// Parses visibility, optional constness, method name, output type, and fallback.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method_attributes = Attribute::parse_outer(input)?;
        let visibility = input.parse()?;
        let is_const = input.peek(Token![const]);
        if is_const {
            input.parse::<Token![const]>()?;
        }
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

/// Operation performed by the generated match arm for one variant.
enum VariantOutput {
    /// Evaluate an ordinary Rust expression.
    Expression(Box<Expr>),
    /// Invoke a closure with every field.
    Call(Box<ExprClosure>),
}

/// Generates an enum method whose body is a match over its variants.
pub fn enum_fn(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<Arguments>(arguments)
        .and_then(|arguments| parse2::<ItemEnum>(item).map(|item| (arguments, item)))
        .and_then(|(arguments, item)| expand(arguments, item));

    result.unwrap_or_else(Error::into_compile_error)
}

/// Removes method-arm expressions and emits the accessor implementation.
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
    if is_const && has_lambda {
        return Err(Error::new_spanned(
            &method,
            "`const enum_fn` cannot use lambda expressions",
        ));
    }
    let constness = is_const.then(|| quote!(const));
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
fn variant_pattern(ident: &Ident, fields: &Fields) -> TokenStream {
    match fields {
        Fields::Unit => quote!(Self::#ident),
        Fields::Unnamed(_) => quote!(Self::#ident(..)),
        Fields::Named(_) => quote!(Self::#ident { .. }),
    }
}

/// Names elided return-reference lifetimes consistently with the receiver.
struct BorrowedReturn<'a> {
    /// Lifetime of the generated shared receiver.
    borrow: &'a Lifetime,
}

impl VisitMut for BorrowedReturn<'_> {
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

    fn visit_type_fn_ptr_mut(&mut self, _: &mut syn::TypeFnPtr) {}

    fn visit_parenthesized_generic_arguments_mut(
        &mut self,
        _: &mut syn::ParenthesizedGenericArguments,
    ) {
    }
}

/// Retains only conditional compilation, including nested `cfg_attr` conditions.
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
