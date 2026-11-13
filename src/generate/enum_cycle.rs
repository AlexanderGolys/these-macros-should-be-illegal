//! Constant cyclic successors in variant declaration order.
use crate::helpers::enum_method::{conditional_attrs, with_method};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Error, Fields, Ident, ItemEnum, Token, Visibility,
    parse::{Parse, ParseStream},
    parse_quote, parse2,
};

/// Parses a unit enum and emits its constant successor method.
pub fn enum_cycle(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<Arguments>(arguments).and_then(|arguments| {
        let item = parse2::<ItemEnum>(item)?;
        expand(arguments, item)
    });
    result.unwrap_or_else(Error::into_compile_error)
}

/// Method metadata supplied by the caller.
struct Arguments {
    attributes: Vec<Attribute>,
    visibility: Visibility,
    name: Ident,
}

impl Parse for Arguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let arguments = Self {
            attributes: input.call(Attribute::parse_outer)?,
            visibility: input.parse()?,
            name: input.parse()?,
        };
        input.parse::<Option<Token![,]>>()?;
        Ok(arguments)
    }
}

/// Pairs each variant with its cyclic successor, skipping disabled candidates.
fn expand(arguments: Arguments, item: ItemEnum) -> syn::Result<TokenStream> {
    let mut variants = Vec::with_capacity(item.variants.len());
    for variant in &item.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(Error::new_spanned(
                &variant.fields,
                "enum_cycle requires unit variants",
            ));
        }
        variants.push((&variant.ident, conditional_attrs(&variant.attrs)));
    }
    let arms = variants.iter().enumerate().map(|(index, (name, attributes))| {
        let mut successors = Vec::new();
        for (successor, conditions) in variants.iter().cycle().skip(index + 1).take(variants.len()) {
            if conditions.is_empty() {
                successors.push(quote!(Self::#successor));
                break;
            }
            successors.push(quote!(#(#conditions)* { return Self::#successor; }));
        }
        if variants.iter().all(|(_, conditions)| !conditions.is_empty()) {
            successors.push(quote!(::core::panic!("no enabled successor")));
        }
        quote!(#(#attributes)* Self::#name => { #[allow(unreachable_code)] { #(#successors)* } })
    });
    let Arguments {
        mut attributes,
        visibility,
        name,
    } = arguments;
    if !attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("doc"))
    {
        attributes.push(parse_quote!(#[allow(missing_docs)]));
    }
    let method = parse_quote! {
        #(#attributes)*
        #visibility const fn #name(&self) -> Self {
            match self {
                #(#arms,)*
                #[allow(unreachable_patterns)]
                _ => ::core::panic!("no enabled successor"),
            }
        }
    };
    Ok(with_method(&item, method))
}

#[cfg(test)]
mod tests {
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
