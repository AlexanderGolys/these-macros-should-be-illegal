//! Stable function-like syntax for objects exposing one selected trait method.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    Attribute, Block, Error, Expr, FnArg, GenericParam, Ident, ItemTrait, Token, TraitItem,
    TraitItemFn,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse_quote, parse2,
    spanned::Spanned,
};

/// Method name reserved as the structural calling convention.
const CALL_METHOD: &str = "__priv_tmsbi_call";

/// The selected method named by `#[callable(...)]`.
struct CallableArguments {
    /// Trait method whose signature and behavior are aliased.
    method: Ident,
}

impl Parse for CallableArguments {
    /// Parses exactly one method identifier.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = Ident::parse_any(input)?;
        if !input.is_empty() {
            return Err(input.error("expected exactly one callable method name"));
        }
        Ok(Self { method })
    }
}

/// One local binding and the macro that invokes it.
struct FunctionBinding {
    /// Whether the generated binding may be mutably borrowed by its call method.
    mutability: Option<Token![mut]>,
    /// Shared name of the value and macro.
    name: Ident,
    /// Expression stored in the local binding.
    value: Expr,
}

impl Parse for FunctionBinding {
    /// Parses `[mut] name = expression`.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mutability = input.parse()?;
        let name = Ident::parse_any(input)?;
        input.parse::<Token![=]>()?;
        let value = input.parse()?;
        input.parse::<Option<Token![;]>>()?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after the callable value"));
        }
        Ok(Self {
            mutability,
            name,
            value,
        })
    }
}

/// Adds a hidden structural-call alias to one method of a trait.
pub(crate) fn callable(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<CallableArguments>(arguments)
        .and_then(|arguments| parse2::<ItemTrait>(item).map(|item| (arguments, item)))
        .and_then(|(arguments, item)| expand_callable(arguments, item));

    result.unwrap_or_else(Error::into_compile_error)
}

/// Creates a local value and a same-name macro forwarding to its call alias.
pub(crate) fn make_fn(input: TokenStream) -> TokenStream {
    parse2::<FunctionBinding>(input)
        .map(expand_function_binding)
        .unwrap_or_else(Error::into_compile_error)
}

/// Appends the selected method's hidden forwarding alias to the trait.
fn expand_callable(arguments: CallableArguments, mut item: ItemTrait) -> syn::Result<TokenStream> {
    let call_name = Ident::new(CALL_METHOD, arguments.method.span());
    if let Some(span) = item.items.iter().find_map(|item| match item {
        TraitItem::Const(constant) if constant.ident == call_name => Some(constant.ident.span()),
        TraitItem::Fn(method) if method.sig.ident == call_name => Some(method.sig.ident.span()),
        _ => None,
    }) {
        return Err(Error::new(
            span,
            format!("`{CALL_METHOD}` is reserved by `callable`"),
        ));
    }

    let selected = item
        .items
        .iter()
        .find_map(|item| match item {
            TraitItem::Fn(method) if method.sig.ident == arguments.method => Some(method.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            Error::new(
                arguments.method.span(),
                format!(
                    "trait `{}` has no method named `{}`",
                    item.ident, arguments.method
                ),
            )
        })?;

    let alias = callable_alias(&item, selected, call_name)?;
    item.items.push(TraitItem::Fn(alias));
    Ok(quote!(#item))
}

/// Copies one method signature and builds its unambiguous forwarding body.
fn callable_alias(
    item: &ItemTrait,
    selected: TraitItemFn,
    call_name: Ident,
) -> syn::Result<TraitItemFn> {
    if selected.sig.variadic.is_some() {
        return Err(Error::new_spanned(
            &selected.sig,
            "a variadic trait method cannot be forwarded by `callable`",
        ));
    }
    if !matches!(selected.sig.inputs.first(), Some(FnArg::Receiver(_))) {
        return Err(Error::new_spanned(
            &selected.sig.ident,
            "the callable method must have a `self` receiver",
        ));
    }

    let mut alias = selected.clone();
    alias.attrs = forwarding_attributes(&selected.attrs);
    alias.attrs.push(parse_quote!(#[doc(hidden)]));
    alias.sig.ident = call_name;
    alias.semi_token = None;

    let mut arguments = Vec::new();
    for (index, input) in alias.sig.inputs.iter_mut().enumerate() {
        let FnArg::Typed(argument) = input else {
            continue;
        };
        let name = format_ident!("__priv_tmsbi_argument_{index}", span = argument.pat.span());
        *argument.pat = parse_quote!(#name);
        arguments.push(name);
    }

    let method = &selected.sig.ident;
    let method_generics = selected
        .sig
        .generics
        .params
        .iter()
        .filter_map(|parameter| match parameter {
            GenericParam::Type(parameter) => {
                let name = &parameter.ident;
                Some(quote!(#name))
            }
            GenericParam::Const(parameter) => {
                let name = &parameter.ident;
                Some(quote!({ #name }))
            }
            GenericParam::Lifetime(_) => None,
        })
        .collect::<Vec<_>>();
    let method_generics = if method_generics.is_empty() {
        TokenStream::new()
    } else {
        quote!(::<#(#method_generics),*>)
    };
    let trait_name = &item.ident;
    let (_, trait_generics, _) = item.generics.split_for_impl();
    let invocation = quote!(
        <Self as #trait_name #trait_generics>::#method #method_generics(
            self #(, #arguments)*
        )
    );
    let invocation = if selected.sig.unsafety.is_some() {
        quote!(unsafe { #invocation })
    } else {
        invocation
    };
    let invocation = if selected.sig.asyncness.is_some() {
        quote!((#invocation).await)
    } else {
        invocation
    };
    alias.default = Some(parse2::<Block>(quote!({ #invocation }))?);
    Ok(alias)
}

/// Retains conditional compilation on the generated alias.
fn forwarding_attributes(attributes: &[Attribute]) -> Vec<Attribute> {
    attributes
        .iter()
        .filter(|attribute| {
            attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
        })
        .cloned()
        .collect()
}

/// Emits the local binding followed by its function-like forwarding macro.
fn expand_function_binding(binding: FunctionBinding) -> TokenStream {
    let FunctionBinding {
        mutability,
        name,
        value,
    } = binding;
    let call_name = Ident::new(CALL_METHOD, Span::call_site());

    quote! {
        let #mutability #name = #value;
        macro_rules! #name {
            ($($arguments:tt)*) => {
                #name.#call_name($($arguments)*)
            };
        }
    }
}

/// Signature preservation, forwarding, and diagnostics.
#[cfg(test)]
mod tests {
    use quote::quote;

    use super::{callable, make_fn};

    /// The alias retains trait and method generics while normalizing patterns.
    #[test]
    fn aliases_a_generic_method() {
        let output = callable(
            quote!(transform),
            quote! {
                trait Transform<'a, T, const N: usize> {
                    fn transform<U>(&self, (value, _): (T, U)) -> &'a T
                    where
                        U: Copy;
                }
            },
        )
        .to_string();

        assert!(output.contains("fn __priv_tmsbi_call < U >"));
        assert!(output.contains("< Self as Transform < 'a , T , N > > :: transform :: < U >"));
        assert!(output.contains("__priv_tmsbi_argument_1"));
    }

    /// The selected method must be callable on an object receiver.
    #[test]
    fn rejects_an_associated_function() {
        let output = callable(
            quote!(create),
            quote!(
                trait Factory {
                    fn create() -> Self;
                }
            ),
        )
        .to_string();

        assert!(output.contains("must have a `self` receiver"));
    }

    /// The private alias cannot overwrite a user-authored trait method.
    #[test]
    fn rejects_the_reserved_method_name() {
        let output = callable(
            quote!(call),
            quote! {
                trait Existing {
                    fn call(&self);
                    fn __priv_tmsbi_call(&self);
                }
            },
        )
        .to_string();

        assert!(output.contains("is reserved by `callable`"));
    }

    /// The binding macro emits exactly one deferred argument repetition.
    #[test]
    fn creates_a_same_name_macro() {
        let output = make_fn(quote!(mut sigma = build())).to_string();

        assert!(output.contains("let mut sigma = build ()"));
        assert!(output.contains("macro_rules ! sigma"));
        assert!(output.contains("sigma . __priv_tmsbi_call"));
    }
}
