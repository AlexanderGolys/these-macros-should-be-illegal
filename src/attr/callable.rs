//! Stable function-like syntax for objects exposing one selected trait method.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Block, Error, FnArg, GenericParam, Ident, ItemTrait, Safety, TraitItem, TraitItemFn,
    parse_quote, parse2, spanned::Spanned,
};

use crate::helpers::arguments::MethodName;

/// Method name reserved as the structural calling convention.
pub(crate) const CALL_METHOD: &str = "__priv_tmsbi_call";

macro_docs! {
    /// Gives one trait method a structural alias that [`make_fn!`](make_fn) calls through.
    ///
    /// No shared callable trait is imposed: the attribute only marks which method of
    /// a user-owned trait the generated same-name macro forwards its arguments to.
    ///
    /// # Examples
    ///
    /// ```
    /// use these_macros_should_be_illegal::{callable, make_fn};
    ///
    /// #[callable(apply)]
    /// trait Action {
    ///     fn apply(&self, point: usize) -> usize;
    /// }
    ///
    /// struct Shift(usize);
    ///
    /// impl Action for Shift {
    ///     fn apply(&self, point: usize) -> usize {
    ///         point + self.0
    ///     }
    /// }
    ///
    /// make_fn!(sigma = Shift(3));
    /// assert_eq!(sigma!(2), 5);
    /// ```
}

/// Adds a hidden structural-call alias to one method of a trait.
pub fn callable(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<MethodName>(arguments)
        .and_then(|arguments| parse2::<ItemTrait>(item).map(|item| (arguments, item)))
        .and_then(|(arguments, item)| expand_callable(arguments, item));

    result.unwrap_or_else(Error::into_compile_error)
}

/// Appends the selected method's hidden forwarding alias to the trait.
fn expand_callable(MethodName(method_name): MethodName, mut item: ItemTrait) -> syn::Result<TokenStream> {
    let call_name = Ident::new(CALL_METHOD, method_name.span());
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
            TraitItem::Fn(method) if method.sig.ident == method_name => Some(method.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            Error::new(
                method_name.span(),
                format!(
                    "trait `{}` has no method named `{}`",
                    item.ident, method_name
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
    let invocation = if matches!(selected.sig.safety, Safety::Unsafe(_)) {
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

/// Signature preservation, forwarding, and diagnostics.
#[cfg(test)]
mod tests {
    use quote::quote;

    use super::callable;

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
}
