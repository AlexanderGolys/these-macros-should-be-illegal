//! Shared emission and conditional attributes for enum methods.
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, ImplItemFn, ItemEnum, Meta, Token, parse_quote_spanned, punctuated::Punctuated,
    spanned::Spanned,
};

/// Emits an enum and its method, preserving generics and conditional compilation.
pub(crate) fn with_method(item: &ItemEnum, method: ImplItemFn) -> TokenStream {
    let ident = &item.ident;
    let (impl_generics, type_generics, where_clause) = item.generics.split_for_impl();
    let attributes = conditional_attrs(&item.attrs);
    quote! {
        #item
        #(#attributes)*
        impl #impl_generics #ident #type_generics #where_clause { #method }
    }
}

/// Retains only conditional compilation, including nested `cfg_attr` conditions.
///
/// - `#[cfg(test)] #[derive(Debug)] #[doc = "..."]` keeps `#[cfg(test)]`.
/// - `#[cfg_attr(unix, cfg(test), derive(Debug))] #[repr(u8)]` keeps
///   `#[cfg_attr(unix, cfg(test))]`.
///
/// ```rust,ignore
/// #[enum_fn(code -> u8, missing = 0)]
/// enum Flag {
///     #[cfg_attr(unix, doc = "Documented only on Unix.")]
///     Shown = 1,
///     #[cfg_attr(not(test), cfg(test))]
///     Hidden = 2,
/// }
/// assert_eq!(Flag::Shown.code(), 1);
/// ```
pub(crate) fn conditional_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter_map(|attribute| {
            let meta = conditional_meta(&attribute.meta)?;
            Some(parse_quote_spanned!(attribute.span()=> #[#meta]))
        })
        .collect()
}

/// Removes variant-only metadata from a conditional attribute before forwarding it.
///
/// Returns `None` when nothing conditional remains to forward:
///
/// - `cfg(test)` stays `cfg(test)`.
/// - `cfg_attr(unix, derive(Debug), cfg(test))` becomes `cfg_attr(unix, cfg(test))`.
/// - `cfg_attr(unix, cfg_attr(test, cfg(debug_assertions)), doc = "...")` becomes
///   `cfg_attr(unix, cfg_attr(test, cfg(debug_assertions)))`.
/// - `cfg_attr(unix, derive(Debug))` and `derive(Debug)` are dropped.
///
/// ```rust,ignore
/// #[enum_fn(code -> u8, missing = 0)]
/// enum Mode {
///     #[cfg_attr(unix, cfg_attr(not(test), cfg(test)), doc = "Unix-only docs.")]
///     Debug = 1,
///     Release = 2,
/// }
/// assert_eq!(Mode::Release.code(), 2);
/// ```
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
