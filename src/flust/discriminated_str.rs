//! Implementation of enum accessors generated from string discriminants.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Attribute, Error, Expr, ExprClosure, ExprLit, Fields, Ident, ItemEnum, Lit, LitStr, Pat,
    PatType, Token, Type, TypeReference,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse2,
};

/// Parsed accessor name and requested output representation.
struct Arguments {
    /// Name of the accessor method generated on the enum.
    method: Ident,
    /// Return representation selected by the attribute arguments.
    output: Output,
    /// Behavior inferred or requested for variants without descriptions.
    missing: MissingDescription,
}

/// Return representation for a generated accessor.
#[derive(Clone, Copy)]
struct Output {
    /// Non-optional value returned when a description exists.
    base: BaseOutput,
    /// Whether the base value is wrapped in `Option`.
    optional: bool,
}

/// Supported non-optional return representations for generated accessors.
#[derive(Clone, Copy)]
enum BaseOutput {
    /// Borrowed static text, retaining `const fn` when every description is fixed.
    StaticStr,
    /// An owned standard-library string.
    String,
    /// A borrowed string slice whose lifetime follows the enum value.
    Str,
}

/// Behavior for variants with neither a discriminant nor an inferred text field.
#[derive(Clone, Copy)]
enum MissingDescription {
    /// Infer an optional accessor and return `None` for the variant.
    None,
    /// Use the variant identifier's spelling as a fixed description.
    Stringify,
}

/// Source of a variant's description.
enum VariantDescription {
    /// A fixed string taken from the variant discriminant.
    Fixed(LitStr),
    /// Runtime text selected from one of the variant's fields.
    Dynamic(DynamicField),
    /// Runtime text computed by invoking a closure with every variant field.
    Closure(ExprClosure),
    /// A variant intentionally lacking a description in optional mode.
    Missing,
}

/// Pattern and text representation for one selected dynamic field.
struct DynamicField {
    /// Match pattern binding the selected field as `value`.
    pattern: TokenStream,
    /// Whether the selected field owns or borrows its text.
    text: TextSource,
}

/// Supported field types for dynamic descriptions.
enum TextSource {
    /// An owned, unqualified `String` field.
    String,
    /// An immutable `&str` field with any explicit or elided lifetime.
    Str,
}

/// Parses the `discriminated_str` attribute's arguments.
impl Parse for Arguments {
    /// Parses a method name, optional ownership type, and optional missing-value override.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = Ident::parse_any(input)?;
        let output = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            parse_output(input.parse()?)?
        } else {
            Output::new(BaseOutput::StaticStr)
        };
        let missing = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let behavior = Ident::parse_any(input)?;
            if behavior != "stringify" {
                return Err(Error::new(
                    behavior.span(),
                    "expected `stringify` as the missing-description behavior",
                ));
            }
            MissingDescription::Stringify
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

/// Generates an enum accessor from fixed or dynamic string descriptions.
pub(crate) fn discriminated_str(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<Arguments>(arguments)
        .and_then(|arguments| parse2::<ItemEnum>(item).map(|item| (arguments, item)))
        .and_then(|(arguments, item)| expand(arguments, item));

    result.unwrap_or_else(Error::into_compile_error)
}

/// Classifies an explicitly requested accessor return type.
fn parse_output(output: Type) -> syn::Result<Output> {
    if is_string_type(&output) {
        return Ok(Output::new(BaseOutput::String));
    }
    if is_elided_str_reference(&output) {
        return Ok(Output::new(BaseOutput::Str));
    }
    Err(Error::new_spanned(
        output,
        "expected `String` or `&str` as the accessor return type; optionality is inferred from the variants",
    ))
}

/// Removes string discriminants and emits the matching accessor implementation.
fn expand(arguments: Arguments, mut item: ItemEnum) -> syn::Result<TokenStream> {
    let Arguments {
        method,
        output,
        missing,
    } = arguments;
    let mut variants = Vec::with_capacity(item.variants.len());

    for variant in &mut item.variants {
        let description = classify_description(
            &variant.ident,
            &variant.fields,
            variant
                .discriminant
                .take()
                .map(|(_, expression)| expression),
            missing,
        )?;

        variants.push((
            variant.ident.clone(),
            variant.attrs.clone(),
            variant.fields.clone(),
            description,
        ));
    }

    let has_dynamic_description = variants.iter().any(|(_, _, _, description)| {
        matches!(
            description,
            VariantDescription::Dynamic(_) | VariantDescription::Closure(_)
        )
    });
    let has_missing_description = variants
        .iter()
        .any(|(_, _, _, description)| matches!(description, VariantDescription::Missing));
    let output = output.with_inferred_shape(has_dynamic_description, has_missing_description);
    let arms: Vec<_> = variants
        .iter()
        .map(|(ident, attrs, fields, description)| {
            accessor_arm(
                ident,
                attrs,
                fields,
                description,
                output.is_owned(),
                output.is_optional(),
            )
        })
        .collect();
    let ident = &item.ident;
    let (impl_generics, type_generics, where_clause) = item.generics.split_for_impl();
    let method_documentation = LitStr::new(
        &format!("Returns this variant's `{method}` value generated by `discriminated_str`."),
        method.span(),
    );
    let method = match (output.base, output.optional) {
        (BaseOutput::StaticStr, false) => quote! {
            #[doc = #method_documentation]
            pub const fn #method(&self) -> &'static str {
                match self {
                    #(#arms),*
                }
            }
        },
        (BaseOutput::StaticStr, true) => quote! {
            #[doc = #method_documentation]
            pub const fn #method(&self) -> ::core::option::Option<&'static str> {
                match self {
                    #(#arms),*
                }
            }
        },
        (BaseOutput::Str, false) => quote! {
            #[doc = #method_documentation]
            pub fn #method(&self) -> &str {
                match self {
                    #(#arms),*
                }
            }
        },
        (BaseOutput::Str, true) => quote! {
            #[doc = #method_documentation]
            pub fn #method(&self) -> ::core::option::Option<&str> {
                match self {
                    #(#arms),*
                }
            }
        },
        (BaseOutput::String, false) => quote! {
            #[doc = #method_documentation]
            pub fn #method(&self) -> ::std::string::String {
                match self {
                    #(#arms),*
                }
            }
        },
        (BaseOutput::String, true) => quote! {
            #[doc = #method_documentation]
            pub fn #method(&self) -> ::core::option::Option<::std::string::String> {
                match self {
                    #(#arms),*
                }
            }
        },
    };

    Ok(quote! {
        #item
        impl #impl_generics #ident #type_generics #where_clause {
            #method
        }
    })
}

/// Classifies one variant's fixed, selected, inferred, or missing description.
fn classify_description(
    variant: &Ident,
    fields: &Fields,
    discriminant: Option<Expr>,
    missing: MissingDescription,
) -> syn::Result<VariantDescription> {
    let Some(expression) = discriminant else {
        if let Some(dynamic) = infer_dynamic_field(variant, fields) {
            return Ok(VariantDescription::Dynamic(dynamic));
        }
        return Ok(match missing {
            MissingDescription::None => VariantDescription::Missing,
            MissingDescription::Stringify => {
                VariantDescription::Fixed(LitStr::new(&variant.unraw().to_string(), variant.span()))
            }
        });
    };

    match expression {
        Expr::Lit(ExprLit {
            lit: Lit::Str(description),
            ..
        }) => Ok(VariantDescription::Fixed(description)),
        Expr::Lit(ExprLit {
            lit: Lit::Int(index),
            ..
        }) => select_unnamed_field(variant, fields, index.base10_parse()?, &index)
            .map(VariantDescription::Dynamic),
        Expr::Path(path) if path.qself.is_none() && path.path.get_ident().is_some() => {
            let field = path.path.get_ident().expect("checked above");
            select_named_field(variant, fields, field).map(VariantDescription::Dynamic)
        }
        Expr::Closure(closure) => {
            validate_closure_arity(fields, &closure)?;
            Ok(VariantDescription::Closure(closure))
        }
        expression => Err(Error::new_spanned(
            expression,
            "expected a string literal, tuple field index, named field, or closure as the variant description",
        )),
    }
}

/// Ensures a description closure accepts one argument for every variant field.
fn validate_closure_arity(fields: &Fields, closure: &ExprClosure) -> syn::Result<()> {
    let field_count = fields.iter().count();
    let input_count = closure.inputs.len();
    if input_count == field_count {
        return Ok(());
    }

    Err(Error::new_spanned(
        closure,
        format!(
            "description closure takes {input_count} arguments, but the variant has {field_count} fields"
        ),
    ))
}

/// Infers a dynamic description from a single `String` or `&str` field.
fn infer_dynamic_field(variant: &Ident, fields: &Fields) -> Option<DynamicField> {
    match fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let field = fields.unnamed.first()?;
            Some(DynamicField {
                pattern: quote!(Self::#variant(value)),
                text: text_source(&field.ty)?,
            })
        }
        Fields::Named(fields) if fields.named.len() == 1 => {
            let field = fields.named.first()?;
            let field_ident = field.ident.as_ref()?;
            Some(DynamicField {
                pattern: quote!(Self::#variant { #field_ident: value }),
                text: text_source(&field.ty)?,
            })
        }
        Fields::Unit | Fields::Unnamed(_) | Fields::Named(_) => None,
    }
}

/// Selects and binds an unnamed string field by its zero-based index.
fn select_unnamed_field(
    variant: &Ident,
    fields: &Fields,
    index: usize,
    selector: &impl quote::ToTokens,
) -> syn::Result<DynamicField> {
    let Fields::Unnamed(fields) = fields else {
        return Err(Error::new_spanned(
            selector,
            "an integer description selector requires a tuple-like variant",
        ));
    };
    let field = fields.unnamed.iter().nth(index).ok_or_else(|| {
        Error::new_spanned(
            selector,
            format!("description field index {index} is out of bounds"),
        )
    })?;
    let text = text_source(&field.ty).ok_or_else(|| unsupported_field_type(&field.ty))?;
    let patterns = fields.unnamed.iter().enumerate().map(|(field_index, _)| {
        if field_index == index {
            quote!(value)
        } else {
            quote!(_)
        }
    });

    Ok(DynamicField {
        pattern: quote!(Self::#variant(#(#patterns),*)),
        text,
    })
}

/// Selects and binds a named string field by identifier.
fn select_named_field(
    variant: &Ident,
    fields: &Fields,
    selector: &Ident,
) -> syn::Result<DynamicField> {
    let Fields::Named(fields) = fields else {
        return Err(Error::new_spanned(
            selector,
            "an identifier description selector requires a struct-like variant",
        ));
    };
    let field = fields
        .named
        .iter()
        .find(|field| field.ident.as_ref() == Some(selector))
        .ok_or_else(|| {
            Error::new_spanned(selector, format!("variant has no field named `{selector}`"))
        })?;
    let text = text_source(&field.ty).ok_or_else(|| unsupported_field_type(&field.ty))?;

    Ok(DynamicField {
        pattern: quote!(Self::#variant { #selector: value, .. }),
        text,
    })
}

/// Returns an error for a selected field that cannot supply string text.
fn unsupported_field_type(ty: &Type) -> Error {
    Error::new_spanned(
        ty,
        "a selected description field must have type `String` or `&str`",
    )
}

/// Classifies a dynamic field as owned or borrowed string text.
fn text_source(ty: &Type) -> Option<TextSource> {
    if is_string_type(ty) {
        Some(TextSource::String)
    } else if is_str_reference(ty) {
        Some(TextSource::Str)
    } else {
        None
    }
}

/// Reports whether a type is an unqualified `String`.
fn is_string_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(ty) if ty.qself.is_none() && ty.path.is_ident("String"))
}

/// Reports whether a type is an immutable `str` reference with any lifetime.
fn is_str_reference(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Reference(reference)
            if reference.mutability.is_none()
                && matches!(reference.elem.as_ref(), Type::Path(path) if path.qself.is_none() && path.path.is_ident("str"))
    )
}

/// Reports whether a type is an immutable lifetime-elided `&str`.
fn is_elided_str_reference(ty: &Type) -> bool {
    matches!(ty, Type::Reference(reference) if reference.lifetime.is_none()) && is_str_reference(ty)
}

/// Builds one match arm for the requested owned, borrowed, or optional output.
fn accessor_arm(
    ident: &Ident,
    attrs: &[Attribute],
    fields: &Fields,
    description: &VariantDescription,
    owned: bool,
    optional: bool,
) -> TokenStream {
    let attrs = conditional_attrs(attrs);
    let (pattern, expression) = match description {
        VariantDescription::Fixed(description) => {
            let value = if owned {
                quote!(::std::string::String::from(#description))
            } else {
                quote!(#description)
            };
            (
                variant_pattern(ident, fields),
                optional_value(value, optional),
            )
        }
        VariantDescription::Dynamic(dynamic) => {
            let value = dynamic.value(owned);
            (dynamic.pattern.clone(), optional_value(value, optional))
        }
        VariantDescription::Closure(closure) => {
            let (pattern, value) = closure_arm(ident, fields, closure);
            (pattern, optional_value(value, optional))
        }
        VariantDescription::Missing => (
            variant_pattern(ident, fields),
            quote!(::core::option::Option::None),
        ),
    };

    quote!(#(#attrs)* #pattern => #expression)
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

        *input = Pat::Type(PatType {
            attrs: Vec::new(),
            pat: Box::new(input.clone()),
            colon_token: Default::default(),
            ty: Box::new(Type::Reference(TypeReference {
                and_token: Default::default(),
                lifetime: None,
                mutability: None,
                elem: Box::new(field.ty.clone()),
            })),
        });
    }
    closure
}

/// Wraps a present description in `Some` when optional output was requested.
fn optional_value(value: TokenStream, optional: bool) -> TokenStream {
    if optional {
        quote!(::core::option::Option::Some(#value))
    } else {
        value
    }
}

/// Provides the expression reading a selected dynamic field.
impl DynamicField {
    /// Borrows or owns the bound `value` according to the accessor output.
    fn value(&self, owned: bool) -> TokenStream {
        match (&self.text, owned) {
            (TextSource::String, false) => quote!(value.as_str()),
            (TextSource::String, true) => quote!(value.clone()),
            (TextSource::Str, false) => quote!(*value),
            (TextSource::Str, true) => quote!(::std::string::String::from(*value)),
        }
    }
}

/// Reports and infers output shape during expansion.
impl Output {
    /// Creates a required output with the selected base representation.
    const fn new(base: BaseOutput) -> Self {
        Self {
            base,
            optional: false,
        }
    }

    /// Infers borrowing and optionality independently from the variant descriptions.
    fn with_inferred_shape(
        mut self,
        has_dynamic_description: bool,
        has_missing_description: bool,
    ) -> Self {
        if has_dynamic_description && matches!(self.base, BaseOutput::StaticStr) {
            self.base = BaseOutput::Str;
        }
        self.optional |= has_missing_description;
        self
    }

    /// Reports whether the accessor allocates an owned `String` when present.
    fn is_owned(self) -> bool {
        matches!(self.base, BaseOutput::String)
    }

    /// Reports whether variants may omit descriptions.
    fn is_optional(self) -> bool {
        self.optional
    }
}

/// Builds a non-binding match pattern for a fixed-description variant.
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
