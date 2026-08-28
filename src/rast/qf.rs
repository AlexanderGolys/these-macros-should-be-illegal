//! Almost-ordinary Rust type qualification for the `qf!` macro.

use core::mem;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, PathArguments, Type, TypePath, parse_quote_spanned, parse2,
    visit_mut::{self, VisitMut},
};

/// Qualifies common unqualified standard-library types throughout one type.
pub(crate) fn qualify_common(input: TokenStream) -> TokenStream {
    let result = parse2::<Type>(input).map(|mut ty| {
        CommonTypeQualifier.visit_type_mut(&mut ty);
        quote!(#ty)
    });

    result.unwrap_or_else(Error::into_compile_error)
}

/// Rewrites recognized single-segment type paths while preserving type arguments.
struct CommonTypeQualifier;

impl VisitMut for CommonTypeQualifier {
    /// Visits nested types before qualifying the path that contains them.
    fn visit_type_path_mut(&mut self, type_path: &mut TypePath) {
        visit_mut::visit_type_path_mut(self, type_path);

        if type_path.qself.is_some()
            || type_path.path.leading_colon.is_some()
            || type_path.path.segments.len() != 1
        {
            return;
        }

        let segment = type_path
            .path
            .segments
            .first_mut()
            .expect("a one-segment path has one segment");
        let span = segment.ident.span();
        let mut replacement: TypePath = match segment.ident.to_string().as_str() {
            "String" => parse_quote_spanned!(span=> ::std::string::String),
            "Box" => parse_quote_spanned!(span=> ::std::boxed::Box),
            "Vec" => parse_quote_spanned!(span=> ::std::vec::Vec),
            "HashMap" => parse_quote_spanned!(span=> ::std::collections::HashMap),
            "HashSet" => parse_quote_spanned!(span=> ::std::collections::HashSet),
            "BTreeMap" => parse_quote_spanned!(span=> ::std::collections::BTreeMap),
            "BTreeSet" => parse_quote_spanned!(span=> ::std::collections::BTreeSet),
            "Option" => parse_quote_spanned!(span=> ::core::option::Option),
            "Arc" => parse_quote_spanned!(span=> ::std::sync::Arc),
            _ => return,
        };
        let arguments = mem::replace(&mut segment.arguments, PathArguments::None);
        replacement
            .path
            .segments
            .last_mut()
            .expect("every replacement path has a final segment")
            .arguments = arguments;
        *type_path = replacement;
    }
}
