//! Generation of the enum of a struct's field names, and of the indexing operator
//! that evaluates a field name on a struct whose fields all hold one type.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Attribute, Data, DataStruct, DeriveInput, Error, Fields, Ident, LitStr, Meta, Token, Type,
    Visibility,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse_quote, parse2,
};

use crate::helpers::ungroup;

/// Prefix of the generated enum's name, read as the logarithm of the struct.
const DEFAULT_PREFIX: &str = "Log";

/// Name of the helper attribute carrying the generated enum's declaration.
const HELPER_ATTRIBUTE: &str = "log";

/// Generates the enum of the field names of one struct, and its indexing operator.
pub fn log(item: TokenStream) -> TokenStream {
    let result = parse2::<DeriveInput>(item).and_then(generate);

    result.unwrap_or_else(Error::into_compile_error)
}

/// The heading of the generated enum's declaration, written without its body.
#[derive(Default)]
struct LogDeclaration {
    /// Attributes of the generated enum.
    attributes: Vec<Attribute>,
    /// Visibility of the generated enum, taken from the struct where absent.
    visibility: Option<Visibility>,
    /// Name of the generated enum, prefixed to the struct's name where absent.
    name: Option<Ident>,
}

/// One field of the struct, which is one variant of the generated enum.
struct LogField {
    /// Name of the field, spelled the same as the variant naming it.
    name: Ident,
    /// Conditional compilation the field and its variant share.
    conditions: Vec<Attribute>,
    /// Documentation of the field, where it is documented.
    documentation: Vec<Attribute>,
    /// The type the field holds.
    ty: Type,
}

impl Parse for LogDeclaration {
    /// Parses the attributes, visibility and name written for the generated enum.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![#]) && input.peek2(Token![!]) {
            return Err(input.error(
                "`log` declares the generated enum with outer attributes, which an item can carry",
            ));
        }

        let attributes = input.call(Attribute::parse_outer)?;
        let visibility = match input.parse::<Visibility>()? {
            Visibility::Inherited => None,
            visibility => Some(visibility),
        };
        input.parse::<Option<Token![enum]>>()?;
        let name = match input.is_empty() {
            true => None,
            false => Some(input.parse()?),
        };
        input.parse::<Option<Token![,]>>()?;

        Ok(Self {
            attributes,
            visibility,
            name,
        })
    }
}

/// Generates the index set of one struct, and its evaluation on it.
fn generate(item: DeriveInput) -> syn::Result<TokenStream> {
    let Data::Struct(DataStruct { fields, .. }) = &item.data else {
        return Err(Error::new_spanned(
            &item.ident,
            format!(
                "`{DEFAULT_PREFIX}` names the fields of a struct; `{}` declares none",
                item.ident
            ),
        ));
    };

    let declaration = declaration(&item)?;
    let fields = log_fields(fields, &item.ident)?;
    let name = declaration.name(&item);
    let attributes = declaration.enum_attributes(&item, &name);
    let visibility = declaration.visibility.unwrap_or_else(|| item.vis.clone());
    let variants = fields.iter().map(|field| field.variant(&item.ident));
    let indexing =
        uniform_type(&fields).map(|shared| index_operator(&item, &fields, &name, shared));

    Ok(quote! {
        #(#attributes)*
        #visibility enum #name {
            #(#variants),*
        }

        #indexing
    })
}

/// Reads the one helper attribute declaring the generated enum, where it is written.
fn declaration(item: &DeriveInput) -> syn::Result<LogDeclaration> {
    let mut written = item
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident(HELPER_ATTRIBUTE));
    let Some(attribute) = written.next() else {
        return Ok(LogDeclaration::default());
    };
    if let Some(repeated) = written.next() {
        return Err(Error::new_spanned(
            repeated,
            format!(
                "`{DEFAULT_PREFIX}` generates one enum, declared by one `{HELPER_ATTRIBUTE}` attribute"
            ),
        ));
    }

    match &attribute.meta {
        Meta::Path(_) => Ok(LogDeclaration::default()),
        meta => parse2(meta.require_list()?.tokens.clone()),
    }
}

/// Generation of the enum of field names from the heading written for it.
impl LogDeclaration {
    /// The name written for the enum, or the struct's name behind the default prefix.
    fn name(&self, item: &DeriveInput) -> Ident {
        match &self.name {
            Some(name) => name.clone(),
            None => Ident::new(
                &format!("{DEFAULT_PREFIX}{}", item.ident.unraw()),
                item.ident.span(),
            ),
        }
    }

    /// The attributes of the enum, completed by the ones its declaration leaves out.
    fn enum_attributes(&self, item: &DeriveInput, name: &Ident) -> Vec<Attribute> {
        let mut attributes: Vec<Attribute> = vec![parse_quote!(
            #[allow(dead_code, non_camel_case_types)]
        )];
        if !self
            .attributes
            .iter()
            .any(|attribute| attribute.path().is_ident("derive"))
        {
            attributes.push(parse_quote!(
                #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
            ));
        }
        if !self.attributes.iter().any(is_documentation) {
            let documentation = LitStr::new(
                &format!("Field names of [`{}`].", item.ident.unraw()),
                name.span(),
            );
            attributes.push(parse_quote!(#[doc = #documentation]));
        }
        attributes.extend(self.attributes.iter().cloned());

        attributes
    }
}

/// The variant naming one field, and the arms evaluating it.
impl LogField {
    /// Reads one named field as the variant naming it.
    fn read(field: &syn::Field, owner: &Ident) -> syn::Result<Self> {
        let name = field.ident.clone().ok_or_else(|| {
            Error::new_spanned(
                field,
                format!(
                    "`{DEFAULT_PREFIX}` names the fields of `{owner}`, which a tuple struct leaves unnamed; its fields are already indexed by position"
                ),
            )
        })?;

        Ok(Self {
            name,
            conditions: field
                .attrs
                .iter()
                .filter(|attribute| is_conditional(attribute))
                .cloned()
                .collect(),
            documentation: field
                .attrs
                .iter()
                .filter(|attribute| is_documentation(attribute))
                .cloned()
                .collect(),
            ty: field.ty.clone(),
        })
    }

    /// The variant naming this field, documented where the field is not.
    fn variant(&self, owner: &Ident) -> TokenStream {
        let Self {
            name,
            conditions,
            documentation,
            ..
        } = self;
        if !documentation.is_empty() {
            return quote!(#(#conditions)* #(#documentation)* #name);
        }

        let generated = LitStr::new(&format!("The `{name}` field of [`{owner}`]."), name.span());
        quote!(#(#conditions)* #[doc = #generated] #name)
    }
}

/// Reads the named fields of one struct as the variants naming them.
fn log_fields(fields: &Fields, owner: &Ident) -> syn::Result<Vec<LogField>> {
    fields
        .iter()
        .map(|field| LogField::read(field, owner))
        .collect()
}

/// The one type every field holds, where the struct has fields and they agree.
fn uniform_type(fields: &[LogField]) -> Option<&Type> {
    let shared = ungroup::ty(&fields.first()?.ty);
    let spelling = shared.to_token_stream().to_string();
    fields
        .iter()
        .all(|field| ungroup::ty(&field.ty).to_token_stream().to_string() == spelling)
        .then_some(shared)
}

/// Generates the evaluation of one field name on the struct, as its indexing operator.
fn index_operator(
    item: &DeriveInput,
    fields: &[LogField],
    name: &Ident,
    shared: &Type,
) -> TokenStream {
    let (generics, arguments, where_clause) = item.generics.split_for_impl();
    let owner = &item.ident;
    let read = fields.iter().map(
        |LogField {
             name: field,
             conditions,
             ..
         }| { quote!(#(#conditions)* #name::#field => &self.#field) },
    );
    let write = fields.iter().map(
        |LogField {
             name: field,
             conditions,
             ..
         }| { quote!(#(#conditions)* #name::#field => &mut self.#field) },
    );

    quote! {
        #[automatically_derived]
        impl #generics ::core::ops::Index<#name> for #owner #arguments #where_clause {
            type Output = #shared;

            fn index(&self, index: #name) -> &Self::Output {
                match index {
                    #(#read),*
                }
            }
        }

        #[automatically_derived]
        impl #generics ::core::ops::IndexMut<#name> for #owner #arguments #where_clause {
            fn index_mut(&mut self, index: #name) -> &mut Self::Output {
                match index {
                    #(#write),*
                }
            }
        }
    }
}

/// Reports whether an attribute is documentation.
fn is_documentation(attribute: &Attribute) -> bool {
    attribute.path().is_ident("doc")
}

/// Reports whether an attribute is conditional compilation, which a variant repeats.
fn is_conditional(attribute: &Attribute) -> bool {
    attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
}

#[cfg(test)]
mod tests {
    //! The generated enum, its indexing operator, and the inputs that generate neither.

    use super::*;

    /// Generates from one struct written as source text.
    fn generated(item: &str) -> String {
        log(item.parse().unwrap()).to_string()
    }

    /// Field names become variants of an enum named after the struct, spelled the same.
    #[test]
    fn names_the_fields_of_a_struct() {
        let output = generated("pub struct BaseGroups { pub Keyword: Style, pub second: Style }");

        assert!(output.contains("pub enum LogBaseGroups"), "{output}");
        assert!(output.contains("Keyword ,"), "{output}");
        assert!(output.contains("second"), "{output}");
        assert!(
            output.contains(r#"# [doc = "The `Keyword` field of [`BaseGroups`]."]"#),
            "{output}"
        );
    }

    /// The struct itself is left to the compiler, which a derive never rewrites.
    #[test]
    fn generates_nothing_of_the_struct() {
        let output = generated("struct BaseGroups { Keyword: Style }");

        assert!(!output.contains("struct BaseGroups"), "{output}");
    }

    /// One shared field type makes the enum an index of the struct.
    #[test]
    fn generates_the_indexing_operator() {
        let output = generated("struct BaseGroups { Keyword: Style, Bg: Style }");

        assert!(
            output.contains("impl :: core :: ops :: Index < LogBaseGroups > for BaseGroups"),
            "{output}"
        );
        assert!(output.contains("type Output = Style"), "{output}");
        assert!(
            output.contains("LogBaseGroups :: Keyword => & self . Keyword"),
            "{output}"
        );
        assert!(
            output.contains("impl :: core :: ops :: IndexMut < LogBaseGroups > for BaseGroups"),
            "{output}"
        );
        assert!(
            output.contains("LogBaseGroups :: Bg => & mut self . Bg"),
            "{output}"
        );
    }

    /// Fields holding different types leave the enum without an indexing operator.
    #[test]
    fn generates_no_operator_for_mixed_types() {
        let output = generated("struct Mixed { count: u8, name: String }");

        assert!(output.contains("enum LogMixed"), "{output}");
        assert!(!output.contains("Index"), "{output}");
    }

    /// The generics and bounds of the struct reach its indexing operator.
    #[test]
    fn keeps_the_generics_of_the_struct() {
        let output =
            generated("struct Pair<'a, T: Copy> where T: Default { left: &'a T, right: &'a T }");

        assert!(
            output.contains("impl < 'a , T : Copy > :: core :: ops :: Index < LogPair > for Pair < 'a , T > where T : Default"),
            "{output}"
        );
        assert!(output.contains("type Output = & 'a T"), "{output}");
    }

    /// The helper attribute names the enum, and carries its attributes and visibility.
    #[test]
    fn takes_the_declared_name_attributes_and_visibility() {
        let output = generated(
            r#"#[log(#[doc = "Axis of a point."] #[derive(Debug)] pub(crate) enum Axis)] struct Point { x: f32, y: f32 }"#,
        );

        assert!(output.contains("pub (crate) enum Axis"), "{output}");
        assert!(
            output.contains(r#"# [doc = "Axis of a point."]"#),
            "{output}"
        );
        assert!(output.contains("# [derive (Debug)]"), "{output}");
        assert!(!output.contains("Clone , Copy"), "{output}");
        assert!(output.contains("Index < Axis > for Point"), "{output}");
    }

    /// A name alone is enough, and the trailing comma of the crate's arguments holds.
    #[test]
    fn takes_a_name_alone() {
        let output = generated("#[log(Axis,)] pub struct Point { x: f32 }");

        assert!(output.contains("pub enum Axis"), "{output}");
    }

    /// The helper attribute may carry nothing at all.
    #[test]
    fn takes_the_bare_helper_attribute() {
        let output = generated("#[log] pub struct Point { x: f32 }");

        assert!(output.contains("pub enum LogPoint"), "{output}");
    }

    /// Without a written derive, the enum derives what an index is expected to have.
    #[test]
    fn derives_the_traits_of_an_index() {
        let output = generated("struct Point { x: f32 }");

        assert!(
            output.contains("# [derive (Clone , Copy , PartialEq , Eq , Hash , Debug)]"),
            "{output}"
        );
        assert!(
            output.contains("# [allow (dead_code , non_camel_case_types)]"),
            "{output}"
        );
    }

    /// The documentation of a field documents the variant naming it.
    #[test]
    fn keeps_the_documentation_of_a_field() {
        let output = generated(r#"struct Point { #[doc = "own"] x: f32 }"#);

        assert!(output.contains(r#"# [doc = "own"] x"#), "{output}");
        assert!(!output.contains("The `x` field"), "{output}");
    }

    /// Conditional compilation written on a field reaches its variant and both arms.
    #[test]
    fn repeats_conditional_compilation() {
        let output = generated("struct Point { #[cfg(all())] x: f32, y: f32 }");

        // The variant and both arms repeat it; the struct is not emitted at all.
        assert_eq!(output.matches("# [cfg (all ())]").count(), 3, "{output}");
    }

    /// A struct without fields is the exponent of the uninhabited enum.
    #[test]
    fn names_no_field_of_a_unit_struct() {
        let unit = generated("struct Nothing;");
        let braced = generated("struct Nothing {}");

        assert!(unit.contains("enum LogNothing { }"), "{unit}");
        assert!(!unit.contains("Index"), "{unit}");
        assert!(braced.contains("enum LogNothing { }"), "{braced}");
    }

    /// Raw identifiers name their variants with the escape they are written with.
    #[test]
    fn names_raw_identifier_fields() {
        let output = generated("struct Keywords { r#type: u8, r#fn: u8 }");

        assert!(output.contains("r#type ,"), "{output}");
        assert!(
            output.contains("LogKeywords :: r#fn => & self . r#fn"),
            "{output}"
        );
    }

    /// The positions of a tuple struct are indexed without being named.
    #[test]
    fn rejects_a_tuple_struct() {
        let output = generated("struct Pair(u8, u8);");

        assert!(output.contains("tuple struct leaves unnamed"), "{output}");
    }

    /// Only a struct declares the fields the enum names.
    #[test]
    fn rejects_an_item_that_is_not_a_struct() {
        let enumeration = generated("enum Other { A, B }");
        let union = generated("union Raw { integer: u32, float: f32 }");

        assert!(enumeration.contains("declares none"), "{enumeration}");
        assert!(union.contains("declares none"), "{union}");
    }

    /// One enum is generated, so one declaration is read.
    #[test]
    fn rejects_a_repeated_helper_attribute() {
        let output = generated("#[log(First)] #[log(Second)] struct Point { x: f32 }");

        assert!(output.contains("one `log` attribute"), "{output}");
    }

    /// The generated enum carries no inner attribute, so its declaration holds none.
    #[test]
    fn rejects_an_inner_attribute() {
        let output = generated(r#"#[log(#![doc = "x"])] struct Point { x: f32 }"#);

        assert!(output.contains("outer attributes"), "{output}");
    }
}
