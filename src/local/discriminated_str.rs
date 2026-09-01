//! Unique string discriminants with same-name delegating constructor macros.

use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Attribute, Error, Expr, ExprLit, Fields, Ident, ItemEnum, Lit, LitStr, Result, Variant,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse2,
};

/// Name of the generated string accessor.
struct Arguments {
    /// Method mapping an enum value to its variant's string.
    method: Ident,
}

impl Parse for Arguments {
    /// Parses exactly one accessor identifier.
    fn parse(input: ParseStream) -> Result<Self> {
        let method = Ident::parse_any(input)?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after the discriminant accessor name"));
        }
        Ok(Self { method })
    }
}

/// Unique string and constructor shape belonging to one enum variant.
struct Discriminant {
    /// Variant identifier used in forward and constructor match arms.
    variant: Ident,
    /// Unique literal assigned to the variant.
    value: LitStr,
    /// Conditional attributes which keep generated arms synchronized.
    attrs: Vec<Attribute>,
    /// Original payload shape used to generate patterns and constructors.
    fields: Fields,
}

/// Generates unique string discriminants and a same-name constructor macro.
pub(crate) fn discriminated_str(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<Arguments>(arguments)
        .and_then(|arguments| parse2::<ItemEnum>(item).map(|item| (arguments, item)))
        .and_then(|(arguments, item)| expand(arguments, item));
    result.unwrap_or_else(Error::into_compile_error)
}

/// Removes string discriminants and emits their two useful directions.
fn expand(arguments: Arguments, mut item: ItemEnum) -> Result<TokenStream> {
    let discriminants = take_discriminants(&mut item)?;
    let enum_ident = &item.ident;
    let method = &arguments.method;
    let (impl_generics, type_generics, where_clause) = item.generics.split_for_impl();
    let forward_arms = discriminants.iter().map(|discriminant| {
        let attrs = &discriminant.attrs;
        let variant = &discriminant.variant;
        let value = &discriminant.value;
        let pattern = variant_pattern(variant, &discriminant.fields);
        quote!(#(#attrs)* #pattern => #value)
    });
    let constructor_arms = discriminants
        .iter()
        .map(|discriminant| constructor_arm(enum_ident, discriminant));
    let method_documentation = LitStr::new(
        &format!("Returns this value's unique `{method}` discriminant."),
        method.span(),
    );
    let constructor_documentation = LitStr::new(
        &format!("Constructs an `{enum_ident}` from one of its string discriminants and payloads."),
        enum_ident.span(),
    );

    Ok(quote! {
        #item

        impl #impl_generics #enum_ident #type_generics #where_clause {
            #[doc = #method_documentation]
            pub const fn #method(&self) -> &'static str {
                match self {
                    #(#forward_arms),*
                }
            }
        }

        #[doc = #constructor_documentation]
        #[allow(unused_macros)]
        macro_rules! #enum_ident {
            #(#constructor_arms);*
        }
    })
}

/// Builds one literal-selected arm of the same-name constructor macro.
fn constructor_arm(enum_ident: &Ident, discriminant: &Discriminant) -> TokenStream {
    let variant = &discriminant.variant;
    let value = &discriminant.value;
    match &discriminant.fields {
        Fields::Unit => quote! {
            (#value $(,)?) => { #enum_ident::#variant }
        },
        Fields::Unnamed(fields) => {
            let arguments: Vec<_> = (0..fields.unnamed.len())
                .map(|index| Ident::new(&format!("__field_{index}"), Span::mixed_site()))
                .collect();
            let matchers = arguments.iter().map(|argument| quote!($#argument:expr));
            let values = arguments.iter().map(|argument| quote!($#argument));
            quote! {
                (#value, #(#matchers),* $(,)?) => {
                    #enum_ident::#variant(#(#values),*)
                }
            }
        }
        Fields::Named(fields) => {
            let arguments: Vec<_> = (0..fields.named.len())
                .map(|index| Ident::new(&format!("__field_{index}"), Span::mixed_site()))
                .collect();
            let names = fields.named.iter().map(|field| {
                field
                    .ident
                    .as_ref()
                    .expect("a named field always has an identifier")
            });
            let matchers = names
                .clone()
                .zip(&arguments)
                .map(|(name, argument)| quote!(#name: $#argument:expr));
            let values = names
                .zip(&arguments)
                .map(|(name, argument)| quote!(#name: $#argument));
            quote! {
                (#value, #(#matchers),* $(,)?) => {
                    #enum_ident::#variant { #(#values),* }
                }
            }
        }
    }
}

/// Extracts and validates one unique string literal for every variant.
fn take_discriminants(item: &mut ItemEnum) -> Result<Vec<Discriminant>> {
    let mut seen = HashMap::<String, LitStr>::new();
    let mut discriminants = Vec::with_capacity(item.variants.len());
    let mut errors: Option<Error> = None;

    for variant in &mut item.variants {
        let result = take_discriminant(variant).and_then(|discriminant| {
            if let Some(previous) = seen.get(&discriminant.value.value()) {
                let mut error = Error::new_spanned(
                    &discriminant.value,
                    format!(
                        "duplicate string discriminant {:?}",
                        discriminant.value.value()
                    ),
                );
                error.combine(Error::new_spanned(previous, "first assigned here"));
                Err(error)
            } else {
                seen.insert(discriminant.value.value(), discriminant.value.clone());
                Ok(discriminant)
            }
        });

        match result {
            Ok(discriminant) => discriminants.push(discriminant),
            Err(error) => {
                if let Some(errors) = &mut errors {
                    errors.combine(error);
                } else {
                    errors = Some(error);
                }
            }
        }
    }

    errors.map_or(Ok(discriminants), Err)
}

/// Extracts one variant's string literal and shape.
fn take_discriminant(variant: &mut Variant) -> Result<Discriminant> {
    let Some((_, expression)) = variant.discriminant.take() else {
        return Err(Error::new_spanned(
            &variant.ident,
            "every variant requires a string literal discriminant",
        ));
    };
    let Expr::Lit(ExprLit {
        lit: Lit::Str(value),
        ..
    }) = expression
    else {
        return Err(Error::new_spanned(
            expression,
            "expected a string literal discriminant",
        ));
    };

    Ok(Discriminant {
        variant: variant.ident.clone(),
        value,
        attrs: conditional_attrs(&variant.attrs).cloned().collect(),
        fields: variant.fields.clone(),
    })
}

/// Builds a non-binding pattern that forgets a variant payload.
fn variant_pattern(ident: &Ident, fields: &Fields) -> TokenStream {
    match fields {
        Fields::Unit => quote!(Self::#ident),
        Fields::Unnamed(_) => quote!(Self::#ident(..)),
        Fields::Named(_) => quote!(Self::#ident { .. }),
    }
}

/// Retains attributes controlling whether the corresponding variant exists.
fn conditional_attrs(attrs: &[Attribute]) -> impl Iterator<Item = &Attribute> {
    attrs.iter().filter(|attribute| {
        attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
    })
}

/// Focused parser and validation tests.
#[cfg(test)]
mod tests {
    use quote::quote;

    use super::discriminated_str;

    /// Duplicate strings cannot select distinct constructors.
    #[test]
    fn rejects_duplicate_discriminants() {
        let output = discriminated_str(
            quote!(name),
            quote! {
                enum Token {
                    First = "same",
                    Second = "same",
                }
            },
        )
        .to_string();

        assert!(output.contains("duplicate string discriminant"));
        assert!(output.contains("first assigned here"));
    }

    /// Every variant must participate in the complete constructor map.
    #[test]
    fn rejects_missing_discriminants() {
        let output = discriminated_str(
            quote!(name),
            quote! {
                enum Token {
                    Present = "present",
                    Missing,
                }
            },
        )
        .to_string();

        assert!(output.contains("every variant requires a string literal discriminant"));
    }
}
