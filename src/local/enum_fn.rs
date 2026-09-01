//! Local enum-syntax extension for methods generated as match expressions.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Attribute, Error, Expr, ExprClosure, ExprConst, ExprLit, Fields, Ident, ItemEnum, Lit, LitStr,
    Member, Pat, PatType, Path, Stmt, Token, Type, Variant,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse_quote_spanned, parse2,
    spanned::Spanned,
};

/// Parsed accessor name and requested output representation.
struct Arguments {
    /// Name of the accessor method generated on the enum.
    method_name: Ident,
    /// Return type before optionality inferred from missing variant arms.
    return_type: Type,
    /// Requested behavior for variants without an arm expression.
    missing: MissingDescription,
}

/// Behavior for variants with neither a discriminant nor an inferred value.
#[derive(Clone, Copy)]
enum MissingDescription {
    /// Return `None` for the variant.
    None,
    /// Use the variant identifier's spelling as a constant value.
    Stringify,
    /// Panic when the accessor reaches the variant.
    Panic,
}

/// Operation performed by the generated match arm for one variant.
enum VariantOutput {
    /// Evaluate an ordinary Rust expression.
    Expression(Box<Expr>),
    /// Read the explicitly selected named or unnamed field.
    Member(Member),
    /// Return `None`.
    None,
    /// Invoke a closure with every field.
    Call(Box<ExprClosure>),
    /// Panic when a variant without an expression is reached.
    Panic,
}

/// Parses the `enum_fn` attribute's arguments.
impl Parse for Arguments {
    /// Parses a method name, required output type, and optional fallback.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = Ident::parse_any(input)?;
        input.parse::<Token![:]>()?;
        let output = input.parse()?;
        let missing = if input.parse::<Option<Token![=]>>()?.is_some() {
            input.parse()?
        } else {
            MissingDescription::None
        };

        if !input.is_empty() {
            return Err(input.error("unexpected tokens after `enum_fn` arguments"));
        }

        Ok(Self {
            method_name: method,
            return_type: output,
            missing,
        })
    }
}

/// Parses the missing-arm fallback following `=`.
impl Parse for MissingDescription {
    /// Accepts `stringify` or `panic`.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let behavior: Path = input.parse()?;
        if behavior.is_ident("stringify") {
            Ok(Self::Stringify)
        } else if behavior.is_ident("panic") {
            Ok(Self::Panic)
        } else {
            Err(Error::new_spanned(
                behavior,
                "expected `stringify` or `panic` as the missing-arm behavior",
            ))
        }
    }
}

/// Generates an enum method whose body is a match over its variants.
pub(crate) fn enum_fn(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<Arguments>(arguments)
        .and_then(|arguments| parse2::<ItemEnum>(item).map(|item| (arguments, item)))
        .and_then(|(arguments, item)| expand(arguments, item));

    result.unwrap_or_else(Error::into_compile_error)
}

/// Removes method-arm expressions and emits the accessor implementation.
fn expand(arguments: Arguments, mut item: ItemEnum) -> syn::Result<TokenStream> {
    let Arguments {
        method_name: method,
        return_type: output,
        missing,
    } = arguments;
    let mut outputs = Vec::with_capacity(item.variants.len());
    let mut has_runtime_output = false;
    let mut optional = false;
    for variant in &mut item.variants {
        let discriminant = variant.discriminant.take();

        let output = match (&variant.fields, discriminant) {
            (
                Fields::Unnamed(fields),
                Some((
                    _,
                    Expr::Lit(ExprLit {
                        lit: Lit::Int(index),
                        ..
                    }),
                )),
            ) => {
                let position = index.base10_parse::<usize>()?;
                let selectable_fields = if fields.unnamed.len() == 1 {
                    match &fields.unnamed.first().expect("checked one field").ty {
                        Type::Tuple(tuple) => tuple.elems.len(),
                        _ => 1,
                    }
                } else {
                    fields.unnamed.len()
                };
                if position >= selectable_fields {
                    return Err(Error::new_spanned(
                        index,
                        format!(
                            "tuple field index `{position}` is out of range for variant `{}` with {} field(s)",
                            variant.ident, selectable_fields,
                        ),
                    ));
                }
                has_runtime_output = true;
                VariantOutput::Member(parse2(quote!(#index))?)
            }
            (Fields::Named(fields), Some((_, Expr::Path(path))))
                if path.qself.is_none()
                    && path.path.get_ident().is_some()
                    && fields
                        .named
                        .iter()
                        .any(|field| field.ident.as_ref() == path.path.get_ident()) =>
            {
                let name = path.path.get_ident().expect("checked above").clone();
                has_runtime_output = true;
                VariantOutput::Member(Member::Named(name))
            }
            (_, Some((_, expression))) => match expression {
                Expr::Closure(closure) => {
                    has_runtime_output = true;
                    VariantOutput::Call(Box::new(closure))
                }
                Expr::Const(expression) => match const_closure(expression) {
                    Ok(closure) => VariantOutput::Call(Box::new(closure)),
                    Err(expression) => VariantOutput::Expression(Box::new(Expr::Const(expression))),
                },
                expression => {
                    has_runtime_output |= !expression_preserves_const(&expression);
                    VariantOutput::Expression(Box::new(expression))
                }
            },

            (_, None) => match missing {
                MissingDescription::None => {
                    optional = true;
                    VariantOutput::None
                }
                MissingDescription::Stringify => {
                    let value =
                        LitStr::new(&variant.ident.unraw().to_string(), variant.ident.span());
                    VariantOutput::Expression(Box::new(Expr::Lit(ExprLit {
                        attrs: Vec::new(),
                        lit: Lit::Str(value),
                    })))
                }
                MissingDescription::Panic => VariantOutput::Panic,
            },
        };
        outputs.push(output);
    }

    let is_const = !has_runtime_output;
    let return_type = return_type(&output, optional);
    let arms = item
        .variants
        .iter()
        .zip(&outputs)
        .map(|(variant, variant_output)| accessor_arm(variant, variant_output, optional))
        .collect::<Vec<_>>();
    let ident = &item.ident;
    let (impl_generics, type_generics, where_clause) = item.generics.split_for_impl();
    let method_documentation = LitStr::new(
        &format!("Returns this variant's `{method}` value generated by `enum_fn`."),
        method.span(),
    );
    let method = if is_const {
        quote! {
            #[doc = #method_documentation]
            pub const fn #method(&self) -> #return_type {
                match self {
                    #(#arms),*
                }
            }
        }
    } else {
        quote! {
            #[doc = #method_documentation]
            pub fn #method(&self) -> #return_type {
                match self {
                    #(#arms),*
                }
            }
        }
    };

    Ok(quote! {
        #item
        impl #impl_generics #ident #type_generics #where_clause {
            #method
        }
    })
}

/// Builds the accessor return type after every variant has been inspected.
fn return_type(output: &Type, optional: bool) -> Type {
    if !optional {
        output.clone()
    } else {
        parse2(quote!(::core::option::Option<#output>))
            .expect("wrapping a parsed type in Option remains a valid type")
    }
}

/// Reports whether an explicit expression asks Rust to const-check the method.
fn expression_preserves_const(expression: &Expr) -> bool {
    matches!(expression, Expr::Const(_) | Expr::Lit(_) | Expr::Path(_))
}

/// Recognizes `const { |...| ... }` as the stable const-closure marker.
fn const_closure(mut expression: ExprConst) -> Result<ExprClosure, ExprConst> {
    let [Stmt::Expr(Expr::Closure(closure), None)] = expression.block.stmts.as_mut_slice() else {
        return Err(expression);
    };
    let mut closure = closure.clone();
    closure.constness = Some(expression.const_token);
    Ok(closure)
}

/// Builds one match arm from its variant and selected operation.
fn accessor_arm(variant: &Variant, output: &VariantOutput, optional: bool) -> TokenStream {
    let attrs = conditional_attrs(&variant.attrs);
    let (pattern, expression) = match output {
        VariantOutput::Expression(value) => (
            variant_pattern(&variant.ident, &variant.fields),
            optional_value(quote!(#value), optional),
        ),
        VariantOutput::Member(member) => {
            let (pattern, source) = member_access(variant, member);
            (pattern, optional_value(source, optional))
        }
        VariantOutput::None => (
            variant_pattern(&variant.ident, &variant.fields),
            quote!(::core::option::Option::None),
        ),
        VariantOutput::Call(closure) => {
            let (pattern, value) = closure_arm(&variant.ident, &variant.fields, closure);
            (pattern, optional_value(value, optional))
        }
        VariantOutput::Panic => {
            let message = LitStr::new(
                &format!("variant `{}` has no generated value", variant.ident),
                variant.ident.span(),
            );
            (
                variant_pattern(&variant.ident, &variant.fields),
                quote!(::core::panic!(#message)),
            )
        }
    };

    quote!(#(#attrs)* #pattern => #expression)
}

/// Builds a compact pattern and borrowed expression for a selected member.
fn member_access(variant: &Variant, member: &Member) -> (TokenStream, TokenStream) {
    match member {
        Member::Unnamed(index) => {
            let ident = &variant.ident;
            if matches!(
                &variant.fields,
                Fields::Unnamed(fields)
                    if fields.unnamed.len() == 1
                        && matches!(&fields.unnamed[0].ty, Type::Tuple(_))
            ) {
                (quote!(Self::#ident { 0: value, .. }), quote!(&value.#index))
            } else {
                (quote!(Self::#ident { #index: value, .. }), quote!(value))
            }
        }
        Member::Named(name) => {
            let ident = &variant.ident;
            (quote!(Self::#ident { #name: value, .. }), quote!(value))
        }
    }
}

/// Lowers a mapping closure with hygienic bindings for every variant field.
fn closure_arm(
    variant: &Ident,
    fields: &Fields,
    closure: &ExprClosure,
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
    let value = if closure.constness.is_some() {
        inline_const_closure(closure, &bindings)
    } else {
        let closure = typed_closure(fields, closure);
        quote!((#closure)(#(#bindings),*))
    };

    (pattern, value)
}

/// Inlines an explicitly const closure so stable Rust never needs to call it.
fn inline_const_closure(closure: &ExprClosure, fields: &[Ident]) -> TokenStream {
    let arguments: Vec<_> = (0..closure.inputs.len())
        .map(|index| Ident::new(&format!("__enum_fn_argument_{index}"), Span::mixed_site()))
        .collect();
    let inputs: Vec<_> = closure.inputs.iter().collect();
    let body = &closure.body;
    let result = Ident::new("__enum_fn_result", Span::mixed_site());
    let expression = match &closure.output {
        syn::ReturnType::Default => quote!(#body),
        syn::ReturnType::Type(_, ty) => quote!({
            let #result: #ty = #body;
            #result
        }),
    };

    quote!({
        let (#(#arguments,)*) = (#(#fields,)*);
        #(let #inputs = #arguments;)*
        #expression
    })
}

/// Supplies field-reference types for closure inputs that the user left untyped.
fn typed_closure(fields: &Fields, closure: &ExprClosure) -> ExprClosure {
    let mut closure = closure.clone();
    for (input, field) in closure.inputs.iter_mut().zip(fields) {
        if matches!(input, Pat::Type(_)) {
            continue;
        }

        let pattern = input.clone();
        let ty = &field.ty;
        let typed: PatType = parse_quote_spanned!(pattern.span()=> #pattern: &#ty);
        *input = Pat::Type(typed);
    }
    closure
}

/// Wraps a present value in `Some` when optional output was inferred.
fn optional_value(value: TokenStream, optional: bool) -> TokenStream {
    if optional {
        quote!(::core::option::Option::Some(#value))
    } else {
        value
    }
}

/// Builds a non-binding match pattern for a variant.
fn variant_pattern(ident: &Ident, fields: &Fields) -> TokenStream {
    match fields {
        Fields::Unit => quote!(Self::#ident),
        Fields::Unnamed(_) => quote!(Self::#ident(..)),
        Fields::Named(_) => quote!(Self::#ident { .. }),
    }
}

/// Retains attributes that can control whether the corresponding variant exists.
fn conditional_attrs(attrs: &[Attribute]) -> impl Iterator<Item = &Attribute> {
    attrs.iter().filter(|attribute| {
        attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
    })
}

/// Parser and diagnostic tests that do not require a downstream crate.
#[cfg(test)]
mod tests {
    use quote::quote;

    use super::enum_fn;

    /// Tuple selectors are checked against the actual variant product.
    #[test]
    fn rejects_an_out_of_range_tuple_selector() {
        let output = enum_fn(
            quote!(value: usize),
            quote! {
                enum Value {
                    Pair(usize, usize) = 2,
                }
            },
        )
        .to_string();

        assert!(output.contains("tuple field index `2` is out of range"));
        assert!(output.contains("variant `Pair` with 2 field(s)"));
    }

    /// Only the two documented missing-arm strategies are accepted.
    #[test]
    fn rejects_an_unknown_missing_arm_strategy() {
        let output = enum_fn(
            quote!(value: usize = default),
            quote! {
                enum Value {
                    Missing,
                }
            },
        )
        .to_string();

        assert!(output.contains("expected `stringify` or `panic`"));
    }

    /// The attribute is deliberately restricted to an enum item.
    #[test]
    fn rejects_a_non_enum_item() {
        let output = enum_fn(
            quote!(value: usize),
            quote! {
                struct Value(usize);
            },
        )
        .to_string();

        assert!(output.contains("expected `enum`"));
    }
}
