//! Local enum-syntax extension for generated discriminant accessors.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Attribute, Error, Expr, ExprClosure, ExprLit, Fields, Ident, ItemEnum, Lit, LitStr, Member,
    Pat, PatType, Path, Token, Type, Variant,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse_quote, parse_quote_spanned, parse2,
    spanned::Spanned,
};

/// Checks whether a type is an unqualified or partially qualified known path.
macro_rules! type_is {
    ($ty:expr, $($($expected:ident)::+)|+ $(,)?) => {
        matches!(
            $ty,
            Type::Path(type_path)
                if type_path.qself.is_none()
                    && $(path_is_subpath_of(
                        &type_path.path,
                        &[$(stringify!($expected)),+],
                    ))||+
        )
    };
}

/// Parsed accessor name and requested output representation.
struct Arguments {
    /// Name of the accessor method generated on the enum.
    method: Ident,
    /// Explicit output type, or `None` when the variants determine it.
    output: Option<BaseOutput>,
    /// Requested behavior for variants without descriptions.
    missing: MissingDescription,
}

/// Supported non-optional return representations for generated accessors.
#[derive(Clone, Copy)]
enum BaseOutput {
    /// Borrowed static text inferred when every value is constant.
    StaticStr,
    /// An owned standard-library string.
    String,
    /// A borrowed string slice whose lifetime follows the enum value.
    Str,
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
    /// Return a string literal.
    Const(LitStr),
    /// Read the explicitly selected named or unnamed field.
    Member(Member),
    /// Read the sole string field inferred from the variant.
    Value,
    /// Return `None`.
    None,
    /// Invoke a closure with every field.
    Call(Box<ExprClosure>),
    /// Panic.
    Panic,
}

/// Parses the `discriminated_str` attribute's arguments.
impl Parse for Arguments {
    /// Parses a method name, optional output type, and optional fallback.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = Ident::parse_any(input)?;
        let output = if input.parse::<Option<Token![:]>>()?.is_some() {
            Some(parse_output(input.parse()?)?)
        } else {
            None
        };
        let missing = if input.parse::<Option<Token![=]>>()?.is_some() {
            input.parse()?
        } else {
            MissingDescription::None
        };

        if !input.is_empty() {
            return Err(input.error("unexpected tokens after `discriminated_str` arguments"));
        }

        Ok(Self {
            method,
            output,
            missing,
        })
    }
}

/// Parses the missing-description fallback following `=`.
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
                "expected `stringify` or `panic` as the missing-description behavior",
            ))
        }
    }
}

/// Generates an enum accessor from its already-parsed variants.
pub(crate) fn discriminated_str(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<Arguments>(arguments)
        .and_then(|arguments| parse2::<ItemEnum>(item).map(|item| (arguments, item)))
        .and_then(|(arguments, item)| expand(arguments, item));

    result.unwrap_or_else(Error::into_compile_error)
}

/// Classifies an explicitly requested accessor return type.
fn parse_output(output: Type) -> syn::Result<BaseOutput> {
    if is_string_type(&output) {
        return Ok(BaseOutput::String);
    }
    if is_elided_str_reference(&output) {
        return Ok(BaseOutput::Str);
    }
    Err(Error::new_spanned(
        output,
        "expected `String` or `&str` as the accessor return type; optionality is inferred from the variants",
    ))
}

/// Removes description discriminants and emits the accessor implementation.
fn expand(arguments: Arguments, mut item: ItemEnum) -> syn::Result<TokenStream> {
    let Arguments {
        method,
        output,
        missing,
    } = arguments;
    let mut outputs = Vec::with_capacity(item.variants.len());
    let mut has_runtime_output = false;
    let mut optional = false;
    for variant in &mut item.variants {
        let discriminant = variant.discriminant.take();

        let output = match (&variant.fields, discriminant) {
            (_, Some((_, expr))) => match expr {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(value),
                    ..
                }) => VariantOutput::Const(value),

                Expr::Lit(ExprLit {
                    lit: Lit::Int(index),
                    ..
                }) => {
                    has_runtime_output = true;
                    VariantOutput::Member(parse2(quote!(#index))?)
                }
                Expr::Path(path) if path.qself.is_none() && path.path.get_ident().is_some() => {
                    let name = path.path.get_ident().expect("checked above").clone();
                    has_runtime_output = true;
                    VariantOutput::Member(Member::Named(name))
                }

                Expr::Closure(closure) => {
                    has_runtime_output = true;
                    VariantOutput::Call(Box::new(closure))
                }
                _ => {
                    return Err(Error::new_spanned(
                        expr,
                        "expected a string literal, tuple field index, named field, or closure as the variant description",
                    ));
                }
            },

            (v, None) => match v {
                Fields::Unnamed(fields)
                    if fields.unnamed.len() == 1 && is_text_type(&fields.unnamed[0].ty) =>
                {
                    has_runtime_output = true;
                    VariantOutput::Value
                }

                Fields::Named(fields)
                    if fields.named.len() == 1 && is_text_type(&fields.named[0].ty) =>
                {
                    has_runtime_output = true;
                    VariantOutput::Value
                }

                _ => match missing {
                    MissingDescription::None => {
                        optional = true;
                        VariantOutput::None
                    }
                    MissingDescription::Stringify => VariantOutput::Const(LitStr::new(
                        &variant.ident.unraw().to_string(),
                        variant.ident.span(),
                    )),
                    MissingDescription::Panic => VariantOutput::Panic,
                },
            },
        };
        outputs.push(output);
    }

    let output = output.unwrap_or(if has_runtime_output {
        BaseOutput::Str
    } else {
        BaseOutput::StaticStr
    });
    let is_const = !has_runtime_output && !output.is_owned();
    let return_type = return_type(output, optional);
    let arms = item
        .variants
        .iter()
        .zip(&outputs)
        .map(|(variant, variant_output)| {
            accessor_arm(variant, variant_output, output.is_owned(), optional)
        })
        .collect::<Vec<_>>();
    let ident = &item.ident;
    let (impl_generics, type_generics, where_clause) = item.generics.split_for_impl();
    let method_documentation = LitStr::new(
        &format!("Returns this variant's `{method}` value generated by `discriminated_str`."),
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
fn return_type(output: BaseOutput, optional: bool) -> Type {
    let ty = match output {
        BaseOutput::StaticStr => parse_quote!(&'static str),
        BaseOutput::Str => parse_quote!(&str),
        BaseOutput::String => parse_quote!(::std::string::String),
    };
    if !optional {
        ty
    } else {
        parse_quote!(::core::option::Option<#ty>)
    }
}

/// Reports whether a field type can supply string text.
fn is_text_type(ty: &Type) -> bool {
    is_string_type(ty) || is_str_reference(ty)
}

/// Reports whether a parsed path is an unqualified or trailing part of a known path.
fn path_is_subpath_of(path: &Path, expected: &[&str]) -> bool {
    path.segments.len() <= expected.len()
        && path
            .segments
            .iter()
            .all(|segment| segment.arguments.is_empty())
        && path
            .segments
            .iter()
            .rev()
            .zip(expected.iter().rev())
            .all(|(actual, expected)| actual.ident == *expected)
}

/// Reports whether a type is a recognized standard string path.
fn is_string_type(ty: &Type) -> bool {
    type_is!(ty, alloc::string::String | std::string::String)
}

/// Reports whether a type is an immutable `str` reference with any lifetime.
fn is_str_reference(ty: &Type) -> bool {
    let Type::Reference(reference) = ty else {
        return false;
    };
    if reference.mutability.is_some() {
        return false;
    }
    type_is!(
        reference.elem.as_ref(),
        core::primitive::str | std::primitive::str
    )
}

/// Reports whether a type is an immutable lifetime-elided `&str`.
fn is_elided_str_reference(ty: &Type) -> bool {
    matches!(ty, Type::Reference(reference) if reference.lifetime.is_none()) && is_str_reference(ty)
}

/// Reports properties of a selected base output.
impl BaseOutput {
    /// Reports whether an accessor allocates or clones an owned string.
    const fn is_owned(self) -> bool {
        matches!(self, Self::String)
    }
}

/// Builds one match arm from its variant and selected operation.
fn accessor_arm(
    variant: &Variant,
    output: &VariantOutput,
    owned: bool,
    optional: bool,
) -> TokenStream {
    let attrs = conditional_attrs(&variant.attrs);
    let (pattern, expression) = match output {
        VariantOutput::Const(value) => {
            let value = if owned {
                quote!(::std::string::String::from(#value))
            } else {
                quote!(#value)
            };
            (
                variant_pattern(&variant.ident, &variant.fields),
                optional_value(value, optional),
            )
        }
        VariantOutput::Member(member) => {
            let (pattern, source) = member_access(variant, member);
            let value = field_value(source, owned);
            (pattern, optional_value(value, optional))
        }
        VariantOutput::Value => {
            let pattern = value_pattern(variant);
            let value = field_value(quote!(value), owned);
            (pattern, optional_value(value, optional))
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
                &format!("variant `{}` has no description", variant.ident),
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

/// Builds a pattern binding an inferred sole value.
fn value_pattern(variant: &Variant) -> TokenStream {
    match &variant.fields {
        Fields::Unnamed(_) => {
            let ident = &variant.ident;
            quote!(Self::#ident(value))
        }
        Fields::Named(fields) => {
            let field = &fields.named[0];
            let name = field
                .ident
                .as_ref()
                .expect("a named field always has an identifier");
            let ident = &variant.ident;
            quote!(Self::#ident { #name: value })
        }
        Fields::Unit => unreachable!("unit variants do not have inferred values"),
    }
}

/// Converts a borrowed text-field expression to the requested representation.
fn field_value(source: TokenStream, owned: bool) -> TokenStream {
    if owned {
        quote!(::std::string::String::from(
            ::core::convert::AsRef::<str>::as_ref(#source)
        ))
    } else {
        source
    }
}

/// Invokes a description closure with hygienic bindings for every variant field.
fn closure_arm(
    variant: &Ident,
    fields: &Fields,
    closure: &ExprClosure,
) -> (TokenStream, TokenStream) {
    let closure = typed_closure(fields, closure);
    let bindings: Vec<_> = (0..fields.iter().count())
        .map(|index| {
            Ident::new(
                &format!("__discriminated_str_field_{index}"),
                Span::mixed_site(),
            )
        })
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
    let value = quote!((#closure)(#(#bindings),*));

    (pattern, value)
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
