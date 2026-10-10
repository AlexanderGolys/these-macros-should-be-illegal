//! Unique string discriminants with same-name delegating constructor macros.

use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};

use crate::helpers::arguments::MethodName;
use crate::helpers::ungroup;
use quote::quote;
use syn::{
    Attribute, Error, Expr, ExprLit, Fields, Ident, ItemEnum, Lit, LitStr, Result, Variant, parse2,
};

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

macro_docs! {
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
}

/// Generates unique string discriminants and a same-name constructor macro.
pub fn discriminated_str(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<MethodName>(arguments)
        .and_then(|arguments| parse2::<ItemEnum>(item).map(|item| (arguments, item)))
        .and_then(|(arguments, item)| expand(arguments, item));
    result.unwrap_or_else(Error::into_compile_error)
}

/// Removes string discriminants and emits their two useful directions.
fn expand(MethodName(method): MethodName, mut item: ItemEnum) -> Result<TokenStream> {
    let discriminants = take_discriminants(&mut item)?;
    let enum_ident = &item.ident;
    let method = &method;
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
        attrs,
    }) = ungroup::expr(&expression)
    else {
        let found = if matches!(ungroup::expr(&expression), Expr::Macro(_)) {
            ", found a macro call; discriminants are read before macros in them expand"
        } else {
            ""
        };
        return Err(Error::new_spanned(
            expression,
            format!("expected a string literal discriminant{found}"),
        ));
    };

    // Only the string is read, so attributes on it would silently have no effect.
    if let Some(attribute) = attrs.first() {
        return Err(Error::new_spanned(
            attribute,
            "attributes on a string discriminant have no effect; put them on the variant",
        ));
    }

    Ok(Discriminant {
        variant: variant.ident.clone(),
        value: value.clone(),
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

    /// Attributes on the literal itself are rejected rather than silently dropped.
    #[test]
    fn rejects_attributes_on_the_discriminant() {
        let output = discriminated_str(quote!(name), quote!(enum E { A = #[cfg(any())] "a" }))
            .to_string();

        assert!(output.contains("put them on the variant"), "{output}");
    }
}
