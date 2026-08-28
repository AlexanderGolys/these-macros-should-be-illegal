//! Implementation of enum accessors generated from string discriminants.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Error, Expr, ExprLit, Fields, Ident, ItemEnum, Lit, LitStr, Token, Type,
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
}

/// Supported return representations for generated accessors.
enum Output {
    /// Borrowed static text, retaining `const fn` when every description is fixed.
    StaticStr,
    /// An owned standard-library string.
    String,
    /// A borrowed string slice whose lifetime follows the enum value.
    Str,
}

/// Source of a variant's description.
enum VariantDescription {
    /// A fixed string taken from the variant discriminant.
    Fixed(LitStr),
    /// The runtime value of the variant's single `String` field.
    Dynamic,
}

/// Parses the `str_disc` attribute's arguments.
impl Parse for Arguments {
    /// Parses an accessor name followed by an optional supported return type.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = Ident::parse_any(input)?;
        let output = if input.is_empty() {
            Output::StaticStr
        } else {
            input.parse::<Token![:]>()?;
            parse_output(input.parse()?)?
        };

        Ok(Self { method, output })
    }
}

/// Generates an enum accessor from fixed or dynamic string descriptions.
pub(crate) fn str_disc(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<Arguments>(arguments)
        .and_then(|arguments| parse2::<ItemEnum>(item).map(|item| (arguments, item)))
        .and_then(|(arguments, item)| expand(arguments, item));

    result.unwrap_or_else(Error::into_compile_error)
}

/// Classifies an explicitly requested accessor return type.
fn parse_output(output: Type) -> syn::Result<Output> {
    match &output {
        Type::Path(output) if output.qself.is_none() && output.path.is_ident("String") => {
            Ok(Output::String)
        }
        Type::Reference(output)
            if output.lifetime.is_none()
                && output.mutability.is_none()
                && matches!(output.elem.as_ref(), Type::Path(path) if path.qself.is_none() && path.path.is_ident("str")) =>
        {
            Ok(Output::Str)
        }
        _ => Err(Error::new_spanned(
            output,
            "expected `String` or `&str` as the accessor return type",
        )),
    }
}

/// Removes string discriminants and emits the matching accessor implementation.
fn expand(arguments: Arguments, mut item: ItemEnum) -> syn::Result<TokenStream> {
    let Arguments { method, output } = arguments;
    let mut variants = Vec::with_capacity(item.variants.len());

    for variant in &mut item.variants {
        let description = match variant.discriminant.take() {
            Some((_, expression)) => {
                let Expr::Lit(ExprLit {
                    lit: Lit::Str(description),
                    ..
                }) = expression
                else {
                    return Err(Error::new_spanned(
                        expression,
                        "string discriminant must be a string literal",
                    ));
                };
                VariantDescription::Fixed(description)
            }
            None if is_dynamic_string_variant(&variant.fields) => VariantDescription::Dynamic,
            None => {
                return Err(Error::new_spanned(
                    &variant.fields,
                    "a variant without a string discriminant must have exactly one `String` field",
                ));
            }
        };

        variants.push((
            variant.ident.clone(),
            variant.attrs.clone(),
            variant.fields.clone(),
            description,
        ));
    }

    let borrowed_arms: Vec<_> = variants
        .iter()
        .map(|(ident, attrs, fields, description)| {
            let attrs = arm_attrs(attrs);
            match description {
                VariantDescription::Fixed(description) => {
                    let pattern = variant_pattern(ident, fields);
                    quote!(#(#attrs)* #pattern => #description)
                }
                VariantDescription::Dynamic => {
                    quote!(#(#attrs)* Self::#ident(value) => value.as_str())
                }
            }
        })
        .collect();
    let owned_arms: Vec<_> = variants
        .iter()
        .map(|(ident, attrs, fields, description)| {
            let attrs = arm_attrs(attrs);
            match description {
                VariantDescription::Fixed(description) => {
                    let pattern = variant_pattern(ident, fields);
                    quote!(#(#attrs)* #pattern => ::std::string::String::from(#description))
                }
                VariantDescription::Dynamic => {
                    quote!(#(#attrs)* Self::#ident(value) => value.clone())
                }
            }
        })
        .collect();
    let has_dynamic_description = variants
        .iter()
        .any(|(_, _, _, description)| matches!(description, VariantDescription::Dynamic));
    let ident = &item.ident;
    let (impl_generics, type_generics, where_clause) = item.generics.split_for_impl();
    let method = match output {
        Output::StaticStr if !has_dynamic_description => quote! {
            pub const fn #method(&self) -> &'static str {
                match self {
                    #(#borrowed_arms),*
                }
            }
        },
        Output::StaticStr | Output::Str => quote! {
            pub fn #method(&self) -> &str {
                match self {
                    #(#borrowed_arms),*
                }
            }
        },
        Output::String => quote! {
            pub fn #method(&self) -> ::std::string::String {
                match self {
                    #(#owned_arms),*
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

/// Reports whether fields are exactly one unnamed, unqualified `String`.
fn is_dynamic_string_variant(fields: &Fields) -> bool {
    let Fields::Unnamed(fields) = fields else {
        return false;
    };
    let Some(field) = fields.unnamed.first().filter(|_| fields.unnamed.len() == 1) else {
        return false;
    };

    matches!(&field.ty, Type::Path(ty) if ty.qself.is_none() && ty.path.is_ident("String"))
}

/// Builds a non-binding match pattern for a fixed-description variant.
fn variant_pattern(ident: &Ident, fields: &Fields) -> TokenStream {
    match fields {
        Fields::Unit => quote!(Self::#ident),
        Fields::Unnamed(_) => quote!(Self::#ident(..)),
        Fields::Named(_) => quote!(Self::#ident { .. }),
    }
}

/// Retains conditional-compilation attributes needed on generated match arms.
fn arm_attrs(attrs: &[Attribute]) -> impl Iterator<Item = &Attribute> {
    attrs.iter().filter(|attribute| {
        attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
    })
}
