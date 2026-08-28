//! Nested algebraic data declarations for the `strutuct!` macro.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Error, Ident, LitStr, Token, Type,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    parse2,
    token::{Brace, Paren},
};

use super::preprocessing::split_config_prefix;

/// One declaration inferred to be either a struct or an enum from its members.
struct Declaration {
    /// Name of the generated Rust type.
    ident: Ident,
    /// Struct fields or enum variants belonging to the declaration.
    body: DeclarationBody,
}

/// The two declaration shapes accepted by the initial DSL.
enum DeclarationBody {
    /// A product type recognized by leading `name: Type` members.
    Struct(Vec<StructField>),
    /// A sum type recognized by variant-shaped members.
    Enum(Vec<EnumVariant>),
}

/// One public field of a generated struct.
struct StructField {
    /// Name retained for the generated Rust field.
    ident: Ident,
    /// Possibly nested or postfix-wrapped field type.
    ty: TypeExpression,
}

/// One variant of a generated enum.
enum EnumVariant {
    /// A payload whose variant name is derived from its type.
    Implicit(Box<TypeExpression>),
    /// An explicitly named tuple-like variant.
    Tuple {
        /// Variant name retained verbatim.
        ident: Ident,
        /// Tuple fields, each of which may contain nested declarations.
        fields: Vec<TypeExpression>,
    },
    /// An explicitly named unit variant.
    Unit(Ident),
}

/// A type together with zero or more postfix ownership wrappers.
struct TypeExpression {
    /// Ordinary Rust type or an inline generated declaration.
    base: TypeBase,
    /// Postfix operators in their source order.
    wrappers: Vec<TypeWrapper>,
}

/// The unwrapped portion of a field or variant type.
enum TypeBase {
    /// Any type already understood by Rust and `syn`.
    Rust(Box<Type>),
    /// A nested declaration that must be hoisted before its parent.
    Nested(Box<Declaration>),
}

/// Supported postfix type constructors.
enum TypeWrapper {
    /// `T?`, lowered to `Option<T>`.
    Option,
    /// `T*`, lowered to `Box<T>`.
    Box,
}

/// Whether a lowered declaration is a product or sum type.
#[derive(Clone, Copy)]
enum DeclarationKind {
    /// A generated struct whose constructor macro accepts fields.
    Struct,
    /// A generated enum whose constructor macro selects a variant path.
    Enum,
}

/// Fully lowered declaration together with its generated items.
struct LoweredDeclaration {
    /// Generated items ordered with dependencies before their consumers.
    items: Vec<TokenStream>,
    /// Name usable when the declaration is embedded as a type.
    ident: Ident,
    /// Algebraic shape used when a parent builds a constructor-macro arm.
    kind: DeclarationKind,
}

/// Fully lowered type expression and nested items collected from it.
struct LoweredType {
    /// Nested declarations and constructor macros emitted before the parent.
    items: Vec<TokenStream>,
    /// Ordinary Rust type replacing the extended input spelling.
    ty: TokenStream,
    /// Identifier used to derive an implicit variant name, when one exists.
    name: Option<Ident>,
    /// Shape of an unwrapped nested declaration, if this type contains one.
    nested_kind: Option<DeclarationKind>,
    /// Whether postfix syntax prevents direct recursive construction.
    wrapped: bool,
}

/// Parses one complete root declaration from a `strutuct!` invocation.
impl Parse for Declaration {
    /// Parses the root name and infers its body from all remaining input.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = Ident::parse_any(input)?;
        parse_declaration(ident, input)
    }
}

/// Parses ordinary and nested types followed by `?` and `*` operators.
impl Parse for TypeExpression {
    /// Parses the base type before consuming left-associative postfix wrappers.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let base = if begins_nested_declaration(input) {
            let ident = Ident::parse_any(input)?;
            let content;
            syn::braced!(content in input);
            TypeBase::Nested(Box::new(parse_declaration(ident, &content)?))
        } else {
            TypeBase::Rust(Box::new(input.parse()?))
        };
        let mut wrappers = Vec::new();

        loop {
            if input.peek(Token![?]) {
                input.parse::<Token![?]>()?;
                wrappers.push(TypeWrapper::Option);
            } else if input.peek(Token![*]) {
                input.parse::<Token![*]>()?;
                wrappers.push(TypeWrapper::Box);
            } else {
                break;
            }
        }

        Ok(Self { base, wrappers })
    }
}

/// Expands one nested algebraic declaration into ordinary Rust items.
pub(crate) fn strutuct(input: TokenStream) -> TokenStream {
    let result = split_config_prefix(input)
        .and_then(|(_, input)| parse2::<Declaration>(input))
        .and_then(lower_declaration)
        .map(|declaration| declaration.items.into_iter().collect());

    result.unwrap_or_else(Error::into_compile_error)
}

/// Parses a declaration body after its name and infers its algebraic shape.
fn parse_declaration(ident: Ident, input: ParseStream) -> syn::Result<Declaration> {
    if input.is_empty() {
        return Err(Error::new(
            ident.span(),
            "an empty declaration is ambiguous; add at least one field or variant",
        ));
    }

    let body = if begins_struct_field(input) {
        DeclarationBody::Struct(parse_struct_fields(input)?)
    } else {
        DeclarationBody::Enum(parse_enum_variants(input)?)
    };

    Ok(Declaration { ident, body })
}

/// Reports whether the next member has the `field: Type` shape.
fn begins_struct_field(input: ParseStream) -> bool {
    let fork = input.fork();
    Ident::parse_any(&fork).is_ok() && fork.peek(Token![:])
}

/// Reports whether the next type is a simple name followed by a declaration body.
fn begins_nested_declaration(input: ParseStream) -> bool {
    let fork = input.fork();
    Ident::parse_any(&fork).is_ok() && fork.peek(Brace)
}

/// Parses comma-separated fields and rejects enum-shaped members in a struct.
fn parse_struct_fields(input: ParseStream) -> syn::Result<Vec<StructField>> {
    let mut fields = Vec::new();

    while !input.is_empty() {
        if !begins_struct_field(input) {
            return Err(input.error("expected a struct field in the form `name: Type`"));
        }

        let ident = Ident::parse_any(input)?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;
        fields.push(StructField { ident, ty });

        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }
    }

    Ok(fields)
}

/// Parses variants, allowing commas to be omitted between delimiter-bounded forms.
fn parse_enum_variants(input: ParseStream) -> syn::Result<Vec<EnumVariant>> {
    let mut variants = Vec::new();

    while !input.is_empty() {
        let variant = if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            let ty = content.parse()?;
            if !content.is_empty() {
                return Err(content.error("an implicit variant accepts exactly one type"));
            }
            EnumVariant::Implicit(Box::new(ty))
        } else {
            let ident = Ident::parse_any(input)?;

            if input.peek(Brace) {
                let content;
                syn::braced!(content in input);
                EnumVariant::Implicit(Box::new(TypeExpression {
                    base: TypeBase::Nested(Box::new(parse_declaration(ident, &content)?)),
                    wrappers: Vec::new(),
                }))
            } else if input.peek(Paren) {
                let content;
                parenthesized!(content in input);
                let mut fields = Vec::new();
                while !content.is_empty() {
                    fields.push(content.parse()?);
                    if !content.is_empty() {
                        content.parse::<Token![,]>()?;
                    }
                }
                EnumVariant::Tuple { ident, fields }
            } else {
                EnumVariant::Unit(ident)
            }
        };

        variants.push(variant);
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
    }

    Ok(variants)
}

/// Lowers a declaration after recursively lowering all nested declarations.
fn lower_declaration(declaration: Declaration) -> syn::Result<LoweredDeclaration> {
    let Declaration { ident, body } = declaration;

    match body {
        DeclarationBody::Struct(fields) => lower_struct(ident, fields),
        DeclarationBody::Enum(variants) => lower_enum(ident, variants),
    }
}

/// Emits a public struct after collecting declarations from every field type.
fn lower_struct(ident: Ident, fields: Vec<StructField>) -> syn::Result<LoweredDeclaration> {
    let mut items = Vec::new();
    let mut lowered_fields = Vec::with_capacity(fields.len());

    for field in fields {
        let lowered = lower_type(field.ty)?;
        items.extend(lowered.items);
        let field_ident = field.ident;
        let field_ty = lowered.ty;
        let field_documentation = LitStr::new(
            &format!("Field `{ident}::{field_ident}` generated by `strutuct!`."),
            field_ident.span(),
        );
        lowered_fields.push(quote! {
            #[doc = #field_documentation]
            pub #field_ident: #field_ty
        });
    }

    let documentation = LitStr::new(
        &format!("Struct generated by `strutuct!` for `{ident}`."),
        ident.span(),
    );
    items.push(quote! {
        #[doc = #documentation]
        pub struct #ident {
            #(#lowered_fields),*
        }
    });
    items.push(emit_struct_macro(&ident));

    Ok(LoweredDeclaration {
        items,
        ident,
        kind: DeclarationKind::Struct,
    })
}

/// Emits a public enum and a recursively delegating constructor macro.
fn lower_enum(ident: Ident, variants: Vec<EnumVariant>) -> syn::Result<LoweredDeclaration> {
    let mut items = Vec::new();
    let mut lowered_variants = Vec::with_capacity(variants.len());
    let mut macro_arms = Vec::new();

    for variant in variants {
        match variant {
            EnumVariant::Implicit(ty) => {
                let lowered = lower_type(*ty)?;
                items.extend(lowered.items);
                let payload_name = lowered.name.ok_or_else(|| {
                    Error::new(
                        ident.span(),
                        "cannot derive an implicit variant name from this type; name the variant explicitly",
                    )
                })?;
                let variant_ident = concatenate(&[&ident, &payload_name]);
                let payload_ty = lowered.ty;
                let variant_documentation = LitStr::new(
                    &format!("Variant `{ident}::{variant_ident}` generated by `strutuct!`."),
                    variant_ident.span(),
                );
                lowered_variants.push(quote! {
                    #[doc = #variant_documentation]
                    #variant_ident(#payload_ty)
                });

                if !lowered.wrapped {
                    match lowered.nested_kind {
                        Some(DeclarationKind::Enum) => {
                            macro_arms.push(quote! {
                                (#payload_name::$($tail:tt)+) => {
                                    #ident::#variant_ident(#payload_name!($($tail)+))
                                };
                            });
                        }
                        Some(DeclarationKind::Struct) => {
                            macro_arms.push(quote! {
                                (#payload_name { $($fields:tt)* }) => {
                                    #ident::#variant_ident(#payload_name! { $($fields)* })
                                };
                            });
                        }
                        None => {
                            macro_arms.push(quote! {
                                (#payload_name($($arguments:tt)*)) => {
                                    #ident::#variant_ident(#payload_ty($($arguments)*))
                                };
                                (#payload_name { $($fields:tt)* }) => {
                                    #ident::#variant_ident(#payload_ty { $($fields)* })
                                };
                                (#payload_name) => {
                                    #ident::#variant_ident(#payload_ty)
                                };
                            });
                        }
                    }
                }
            }
            EnumVariant::Tuple {
                ident: variant_ident,
                fields,
            } => {
                let mut lowered_fields = Vec::with_capacity(fields.len());
                for field in fields {
                    let lowered = lower_type(field)?;
                    items.extend(lowered.items);
                    lowered_fields.push(lowered.ty);
                }
                let variant_documentation = LitStr::new(
                    &format!("Variant `{ident}::{variant_ident}` generated by `strutuct!`."),
                    variant_ident.span(),
                );
                lowered_variants.push(quote! {
                    #[doc = #variant_documentation]
                    #variant_ident(#(#lowered_fields),*)
                });
            }
            EnumVariant::Unit(variant_ident) => {
                let variant_documentation = LitStr::new(
                    &format!("Variant `{ident}::{variant_ident}` generated by `strutuct!`."),
                    variant_ident.span(),
                );
                lowered_variants.push(quote! {
                    #[doc = #variant_documentation]
                    #variant_ident
                });
            }
        }
    }

    let documentation = LitStr::new(
        &format!("Enum generated by `strutuct!` for `{ident}`."),
        ident.span(),
    );
    items.push(quote! {
        #[doc = #documentation]
        pub enum #ident {
            #(#lowered_variants),*
        }
    });
    items.push(emit_enum_macro(&ident, &macro_arms));

    Ok(LoweredDeclaration {
        items,
        ident,
        kind: DeclarationKind::Enum,
    })
}

/// Lowers a type expression and marks wrapped recursive edges as terminal.
fn lower_type(expression: TypeExpression) -> syn::Result<LoweredType> {
    let TypeExpression { base, wrappers } = expression;
    let mut lowered = match base {
        TypeBase::Rust(ty) => {
            let ty = *ty;
            LoweredType {
                name: type_name(&ty),
                ty: ty.into_token_stream(),
                items: Vec::new(),
                nested_kind: None,
                wrapped: false,
            }
        }
        TypeBase::Nested(declaration) => {
            let declaration = lower_declaration(*declaration)?;
            LoweredType {
                ty: declaration.ident.to_token_stream(),
                name: Some(declaration.ident),
                items: declaration.items,
                nested_kind: Some(declaration.kind),
                wrapped: false,
            }
        }
    };

    if !wrappers.is_empty() {
        lowered.wrapped = true;
    }
    for wrapper in wrappers {
        let ty = lowered.ty;
        lowered.ty = match wrapper {
            TypeWrapper::Option => quote!(::core::option::Option<#ty>),
            TypeWrapper::Box => quote!(::std::boxed::Box<#ty>),
        };
    }

    Ok(lowered)
}

/// Extracts a deterministic suffix from a path-like Rust type.
fn type_name(ty: &Type) -> Option<Ident> {
    match ty {
        Type::Path(path) if path.qself.is_none() => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.clone()),
        Type::Group(group) => type_name(&group.elem),
        Type::Paren(parenthesized) => type_name(&parenthesized.elem),
        _ => None,
    }
}

/// Emits a same-name macro that constructs a generated struct literal.
fn emit_struct_macro(ident: &Ident) -> TokenStream {
    let documentation = LitStr::new(
        &format!("Constructs a `{ident}` value generated by `strutuct!`."),
        ident.span(),
    );
    quote! {
        #[doc = #documentation]
        #[allow(unused_macros)]
        macro_rules! #ident {
            ($($fields:tt)*) => {
                #ident { $($fields)* }
            };
        }
    }
}

/// Emits a same-name macro whose specialized arms fold nested variant paths.
fn emit_enum_macro(ident: &Ident, specialized_arms: &[TokenStream]) -> TokenStream {
    let documentation = LitStr::new(
        &format!("Constructs an `{ident}` value generated by `strutuct!`."),
        ident.span(),
    );
    quote! {
        #[doc = #documentation]
        #[allow(unused_macros)]
        macro_rules! #ident {
            #(#specialized_arms)*
            ($($variant:tt)+) => {
                #ident::$($variant)+
            };
        }
    }
}

/// Concatenates identifiers while retaining the final source segment's span.
fn concatenate(parts: &[&Ident]) -> Ident {
    let spelling = parts
        .iter()
        .map(|part| part.unraw().to_string())
        .collect::<String>();
    let span = parts
        .last()
        .map_or_else(Span::call_site, |part| part.span());
    Ident::new(&spelling, span)
}

#[cfg(test)]
mod tests {
    //! Parser and lowering tests for the nested algebraic declaration syntax.

    use super::*;

    /// Expands source text and normalizes it through Rust's file parser.
    fn expand(input: &str) -> syn::File {
        let tokens: TokenStream = input.parse().unwrap();
        syn::parse2(strutuct(tokens)).unwrap()
    }

    /// Normalizes ordinary Rust source for structural comparison.
    fn rust(input: &str) -> syn::File {
        syn::parse_str(input).unwrap()
    }

    /// Compares generated and expected items without depending on whitespace.
    fn assert_expands_to(input: &str, expected: &str) {
        assert_eq!(
            expand(input).into_token_stream().to_string(),
            rust(expected).into_token_stream().to_string()
        );
    }

    /// Hoists a nested enum before the struct that refers to it.
    #[test]
    fn lowers_nested_enum_in_struct_field() {
        assert_expands_to(
            "S a: A { A1, A2, A3 }, b: B,",
            r#"
                #[doc = "Enum generated by `strutuct!` for `A`."]
                pub enum A {
                    #[doc = "Variant `A::A1` generated by `strutuct!`."]
                    A1,
                    #[doc = "Variant `A::A2` generated by `strutuct!`."]
                    A2,
                    #[doc = "Variant `A::A3` generated by `strutuct!`."]
                    A3
                }
                #[doc = "Constructs an `A` value generated by `strutuct!`."]
                #[allow(unused_macros)]
                macro_rules! A {
                    ($($variant:tt)+) => { A:: $($variant)+ };
                }
                #[doc = "Struct generated by `strutuct!` for `S`."]
                pub struct S {
                    #[doc = "Field `S::a` generated by `strutuct!`."]
                    pub a: A,
                    #[doc = "Field `S::b` generated by `strutuct!`."]
                    pub b: B
                }
                #[doc = "Constructs a `S` value generated by `strutuct!`."]
                #[allow(unused_macros)]
                macro_rules! S {
                    ($($fields:tt)*) => { S { $($fields)* } };
                }
            "#,
        );
    }

    /// Generates same-name macros that recursively construct nested enum chains.
    #[test]
    fn generates_direct_and_transitive_constructors() {
        let expanded = expand("Expr Unary { (Pref), (Post) } (Bin) LitStr(String) Null")
            .into_token_stream()
            .to_string();

        for expected in [
            "pub enum Unary",
            "UnaryPref (Pref)",
            "UnaryPost (Post)",
            "pub enum Expr",
            "ExprUnary (Unary)",
            "ExprBin (Bin)",
            "LitStr (String)",
            "macro_rules ! Unary",
            "Unary :: UnaryPref (Pref ($ ($ arguments) *))",
            "macro_rules ! Expr",
            "Expr :: ExprUnary (Unary ! ($ ($ tail) +))",
            "Expr :: ExprBin (Bin ($ ($ arguments) *))",
        ] {
            assert!(
                expanded.contains(expected),
                "missing `{expected}` in `{expanded}`"
            );
        }
    }

    /// Lowers postfix option and box operators from left to right.
    #[test]
    fn lowers_postfix_type_operators() {
        let expanded = expand("S optional: A?, nested: B?*,")
            .into_token_stream()
            .to_string();

        assert!(expanded.contains("Option < A >"));
        assert!(expanded.contains("Box < :: core :: option :: Option < B > >"));
    }
}
