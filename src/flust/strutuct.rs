//! Nested algebraic data declarations for the `strutuct!` macro.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Attribute, Error, Ident, LitBool, LitStr, Path, Token, Type,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    parse_quote_spanned, parse2,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Brace, Paren},
};

use super::preprocessing::split_config_prefix;

/// One complete invocation with declaration-local lowering options.
struct Invocation {
    /// Options consumed from the root declaration's `strutuct` attribute.
    options: Options,
    /// Root algebraic declaration.
    declaration: Declaration,
}

/// Optional behavior selected for one declaration family.
#[derive(Clone, Copy)]
struct Options {
    /// Whether non-unit enum variants carry one explicit product value.
    product_variants: bool,
}

/// Supplies the default algebraic lowering behavior.
impl Default for Options {
    /// Enables unary product variants unless the declaration opts out.
    fn default() -> Self {
        Self {
            product_variants: true,
        }
    }
}

/// One declaration inferred to be either a struct or an enum from its members.
struct Declaration {
    /// Ordinary Rust attributes forwarded to the generated declaration.
    attrs: Vec<Attribute>,
    /// Name of the generated Rust type.
    ident: Ident,
    /// Struct fields or enum variants belonging to the declaration.
    body: DeclarationBody,
}

/// The three declaration shapes inferred from the DSL body.
enum DeclarationBody {
    /// A product type recognized by leading `name: Type` members.
    Struct(Vec<StructField>),
    /// A tuple struct recognized by one parenthesized product of at least two types.
    Tuple(Vec<TypeExpression>),
    /// A sum type recognized by variant-shaped members.
    Enum(Vec<ParsedEnumVariant>),
}

/// One public field of a generated struct.
struct StructField {
    /// Ordinary Rust attributes forwarded to the generated field.
    attrs: Vec<Attribute>,
    /// Name retained for the generated Rust field.
    ident: Ident,
    /// Possibly nested or postfix-wrapped field type.
    ty: TypeExpression,
}

/// One parsed enum variant together with attributes and a local behavior override.
struct ParsedEnumVariant {
    /// Ordinary Rust attributes forwarded to the generated variant.
    attrs: Vec<Attribute>,
    /// Per-variant override for unary product lowering.
    product_variants: Option<bool>,
    /// Syntactic form of the variant.
    kind: EnumVariant,
}

/// Syntactic form of one generated enum variant.
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
    /// A named variant whose payload type is synthesized from its body.
    Generated {
        /// Variant name retained verbatim.
        ident: Ident,
        /// Generated payload declaration.
        declaration: Box<Declaration>,
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
    /// A generated tuple struct whose constructor macro accepts positional fields.
    Tuple,
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

/// Parses one complete `strutuct!` invocation.
impl Parse for Invocation {
    /// Parses declaration attributes, options, the root name, and its body.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = input.call(Attribute::parse_outer)?;
        let options = Options {
            product_variants: take_product_variants(&mut attrs)?.unwrap_or(true),
        };
        let ident = Ident::parse_any(input)?;
        let declaration = parse_declaration(attrs, ident, input)?;
        Ok(Self {
            options,
            declaration,
        })
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
            TypeBase::Nested(Box::new(parse_declaration(Vec::new(), ident, &content)?))
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
        .and_then(|(_, input)| parse2::<Invocation>(input))
        .and_then(|mut invocation| {
            let attrs = propagated_declaration_attrs(&invocation.declaration.attrs);
            propagate_declaration_attrs(&mut invocation.declaration.body, &attrs);
            lower_declaration(invocation.declaration, invocation.options)
        })
        .map(|declaration| declaration.items.into_iter().collect());

    result.unwrap_or_else(Error::into_compile_error)
}

/// Selects root attributes that generated nested declarations must also carry.
fn propagated_declaration_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|attribute| {
            attribute.path().is_ident("derive")
                || attribute.path().is_ident("cfg")
                || attribute.path().is_ident("cfg_attr")
        })
        .cloned()
        .collect()
}

/// Reports whether an attribute declares one or more derived traits.
fn is_derive(attribute: &Attribute) -> bool {
    attribute.path().is_ident("derive")
}

/// Reports whether an ordinary derive list explicitly contains `Default`.
fn derives_default(attribute: &Attribute) -> bool {
    is_derive(attribute)
        && attribute
            .parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
            .is_ok_and(|derives| derives.iter().any(|derive| derive.is_ident("Default")))
}

/// Applies inherited declaration attributes to every nested generated type.
fn propagate_declaration_attrs(body: &mut DeclarationBody, attrs: &[Attribute]) {
    match body {
        DeclarationBody::Struct(fields) => {
            for field in fields {
                propagate_type_attrs(&mut field.ty, attrs);
            }
        }
        DeclarationBody::Tuple(fields) => {
            for field in fields {
                propagate_type_attrs(field, attrs);
            }
        }
        DeclarationBody::Enum(variants) => {
            for variant in variants {
                match &mut variant.kind {
                    EnumVariant::Implicit(ty) => propagate_type_attrs(ty, attrs),
                    EnumVariant::Tuple { fields, .. } => {
                        for field in fields {
                            propagate_type_attrs(field, attrs);
                        }
                    }
                    EnumVariant::Generated { declaration, .. } => {
                        propagate_nested_declaration_attrs(declaration, attrs);
                    }
                    EnumVariant::Unit(_) => {}
                }
            }
        }
    }
}

/// Applies inherited attributes when a type expression defines a nested type.
fn propagate_type_attrs(expression: &mut TypeExpression, attrs: &[Attribute]) {
    if let TypeBase::Nested(declaration) = &mut expression.base {
        propagate_nested_declaration_attrs(declaration, attrs);
    }
}

/// Adds inherited attributes to one declaration and recursively visits its body.
fn propagate_nested_declaration_attrs(declaration: &mut Declaration, attrs: &[Attribute]) {
    let overrides_derive = declaration.attrs.iter().any(is_derive);
    let overrides_default = declaration.attrs.iter().any(derives_default);
    let mut retained_default = false;
    let mut inherited = Vec::new();
    for attribute in attrs {
        if !is_derive(attribute) || !overrides_derive {
            inherited.push(attribute.clone());
        } else if !overrides_default && !retained_default && derives_default(attribute) {
            inherited.push(parse_quote_spanned!(attribute.span()=> #[derive(Default)]));
            retained_default = true;
        }
    }
    for attribute in inherited.iter().rev() {
        if !declaration.attrs.iter().any(|existing| {
            existing.to_token_stream().to_string() == attribute.to_token_stream().to_string()
        }) {
            declaration.attrs.insert(0, attribute.clone());
        }
    }
    let mut descendants = attrs
        .iter()
        .filter(|attribute| !is_derive(attribute))
        .cloned()
        .collect::<Vec<_>>();
    descendants.extend(
        declaration
            .attrs
            .iter()
            .filter(|attribute| is_derive(attribute))
            .cloned(),
    );
    propagate_declaration_attrs(&mut declaration.body, &descendants);
}

/// Consumes `strutuct` configuration attributes and retains ordinary Rust attributes.
fn take_product_variants(attrs: &mut Vec<Attribute>) -> syn::Result<Option<bool>> {
    let mut product_variants = None;
    let mut configured = false;
    let mut retained = Vec::with_capacity(attrs.len());

    for attribute in attrs.drain(..) {
        if !attribute.path().is_ident("strutuct") {
            retained.push(attribute);
            continue;
        }
        if configured {
            return Err(Error::new_spanned(
                attribute,
                "duplicate `strutuct` configuration attribute",
            ));
        }
        configured = true;
        attribute.parse_nested_meta(|meta| {
            if !meta.path.is_ident("product_variants") {
                return Err(meta.error("unknown `strutuct` option"));
            }
            if product_variants.is_some() {
                return Err(meta.error("duplicate `product_variants` option"));
            }
            product_variants = Some(meta.value()?.parse::<LitBool>()?.value);
            Ok(())
        })?;
    }

    *attrs = retained;
    Ok(product_variants)
}

/// Parses a declaration body after its name and infers its algebraic shape.
fn parse_declaration(
    attrs: Vec<Attribute>,
    ident: Ident,
    input: ParseStream,
) -> syn::Result<Declaration> {
    if input.is_empty() {
        return Err(Error::new(
            ident.span(),
            "an empty declaration is ambiguous; add at least one field or variant",
        ));
    }

    let body = if begins_struct_field(input) {
        DeclarationBody::Struct(parse_struct_fields(input)?)
    } else if begins_tuple_declaration(input)? {
        DeclarationBody::Tuple(parse_tuple_fields(input)?)
    } else {
        DeclarationBody::Enum(parse_enum_variants(&ident, input)?)
    };

    Ok(Declaration { attrs, ident, body })
}

/// Reports whether the complete body is one product containing at least two types.
fn begins_tuple_declaration(input: ParseStream) -> syn::Result<bool> {
    let fork = input.fork();
    if !fork.peek(Paren) {
        return Ok(false);
    }

    let content;
    parenthesized!(content in fork);
    if fork.peek(Token![,]) && fork.parse::<Token![,]>().is_err() {
        return Ok(false);
    }

    let fields = parse_type_list(&content)?;
    Ok(fork.is_empty() && fields.len() >= 2 && content.is_empty())
}

/// Reports whether the next member has the `field: Type` shape.
fn begins_struct_field(input: ParseStream) -> bool {
    let fork = input.fork();
    if fork.call(Attribute::parse_outer).is_err() {
        return false;
    }
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

        let mut attrs = input.call(Attribute::parse_outer)?;
        let ident = Ident::parse_any(input)?;
        input.parse::<Token![:]>()?;
        let mut ty: TypeExpression = input.parse()?;
        if let TypeBase::Nested(declaration) = &mut ty.base {
            let mut field_attrs = Vec::with_capacity(attrs.len());
            for attribute in attrs {
                if is_derive(&attribute) {
                    declaration.attrs.push(attribute);
                } else {
                    field_attrs.push(attribute);
                }
            }
            attrs = field_attrs;
        } else if let Some(derive) = attrs.iter().find(|attribute| is_derive(attribute)) {
            return Err(Error::new_spanned(
                derive,
                "a local derive override requires an inline generated field type",
            ));
        }
        fields.push(StructField { attrs, ident, ty });

        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }
    }

    Ok(fields)
}

/// Parses the positional fields of one tuple declaration.
fn parse_tuple_fields(input: ParseStream) -> syn::Result<Vec<TypeExpression>> {
    let content;
    parenthesized!(content in input);
    let fields = parse_type_list(&content)?;
    if fields.len() < 2 {
        return Err(content.error("a tuple declaration requires at least two fields"));
    }
    if input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
    }
    if !input.is_empty() {
        return Err(input.error("unexpected tokens after tuple declaration"));
    }
    Ok(fields)
}

/// Parses a comma-separated list of type expressions.
fn parse_type_list(input: ParseStream) -> syn::Result<Vec<TypeExpression>> {
    let mut fields = Vec::new();
    while !input.is_empty() {
        fields.push(input.parse()?);
        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }
    }
    Ok(fields)
}

/// Parses variants, allowing commas to be omitted between delimiter-bounded forms.
fn parse_enum_variants(parent: &Ident, input: ParseStream) -> syn::Result<Vec<ParsedEnumVariant>> {
    let mut variants = Vec::new();

    while !input.is_empty() {
        let mut attrs = input.call(Attribute::parse_outer)?;
        let product_variants = take_product_variants(&mut attrs)?;
        let kind = if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            let mut fields = parse_type_list(&content)?;
            if fields.len() != 1 {
                return Err(content.error("an implicit variant accepts exactly one type"));
            }
            let mut ty = fields.pop().expect("checked one field above");
            if input.peek(Brace) {
                let definition;
                syn::braced!(definition in input);
                ty = define_type(ty, &definition)?;
            }
            EnumVariant::Implicit(Box::new(ty))
        } else {
            let ident = Ident::parse_any(input)?;

            if input.peek(Brace) {
                let content;
                syn::braced!(content in input);
                let generated_ident = concatenate(&[parent, &ident]);
                EnumVariant::Generated {
                    ident,
                    declaration: Box::new(parse_declaration(
                        Vec::new(),
                        generated_ident,
                        &content,
                    )?),
                }
            } else if input.peek(Paren) {
                let content;
                parenthesized!(content in input);
                let mut fields = parse_type_list(&content)?;
                if input.peek(Brace) {
                    if fields.len() != 1 {
                        return Err(
                            content.error("a generated payload requires exactly one type name")
                        );
                    }
                    let definition;
                    syn::braced!(definition in input);
                    let ty = fields.pop().expect("checked one field above");
                    fields.push(define_type(ty, &definition)?);
                }
                EnumVariant::Tuple { ident, fields }
            } else {
                EnumVariant::Unit(ident)
            }
        };

        variants.push(ParsedEnumVariant {
            attrs,
            product_variants,
            kind,
        });
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
    }

    Ok(variants)
}

/// Replaces one explicit type name with the declaration defined by the following body.
fn define_type(expression: TypeExpression, body: ParseStream) -> syn::Result<TypeExpression> {
    let TypeExpression { base, wrappers } = expression;
    if !wrappers.is_empty() {
        return Err(body.error("a generated type name cannot use postfix wrappers"));
    }
    let TypeBase::Rust(ty) = base else {
        return Err(body.error("expected an undeclared type name before this body"));
    };
    let Type::Path(path) = ty.as_ref() else {
        return Err(Error::new_spanned(ty, "expected a single type identifier"));
    };
    if path.qself.is_some() || path.path.leading_colon.is_some() || path.path.segments.len() != 1 {
        return Err(Error::new_spanned(ty, "expected a single type identifier"));
    }
    let segment = &path.path.segments[0];
    if !segment.arguments.is_empty() {
        return Err(Error::new_spanned(ty, "expected a single type identifier"));
    }

    Ok(TypeExpression {
        base: TypeBase::Nested(Box::new(parse_declaration(
            Vec::new(),
            segment.ident.clone(),
            body,
        )?)),
        wrappers: Vec::new(),
    })
}

/// Lowers a declaration after recursively lowering all nested declarations.
fn lower_declaration(
    declaration: Declaration,
    options: Options,
) -> syn::Result<LoweredDeclaration> {
    let Declaration { attrs, ident, body } = declaration;

    match body {
        DeclarationBody::Struct(fields) => lower_struct(attrs, ident, fields, options),
        DeclarationBody::Tuple(fields) => lower_tuple(attrs, ident, fields, options),
        DeclarationBody::Enum(variants) => lower_enum(attrs, ident, variants, options),
    }
}

/// Emits a public struct after collecting declarations from every field type.
fn lower_struct(
    attrs: Vec<Attribute>,
    ident: Ident,
    fields: Vec<StructField>,
    options: Options,
) -> syn::Result<LoweredDeclaration> {
    let mut items = Vec::new();
    let mut lowered_fields = Vec::with_capacity(fields.len());

    for field in fields {
        let lowered = lower_type(field.ty, options)?;
        items.extend(lowered.items);
        let field_attrs = field.attrs;
        let field_ident = field.ident;
        let field_ty = lowered.ty;
        let field_documentation = LitStr::new(
            &format!("Field `{ident}::{field_ident}` generated by `strutuct!`."),
            field_ident.span(),
        );
        lowered_fields.push(quote! {
            #(#field_attrs)*
            #[doc = #field_documentation]
            pub #field_ident: #field_ty
        });
    }

    let documentation = LitStr::new(
        &format!("Struct generated by `strutuct!` for `{ident}`."),
        ident.span(),
    );
    items.push(quote! {
        #(#attrs)*
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

/// Emits a public tuple struct after collecting declarations from every field type.
fn lower_tuple(
    attrs: Vec<Attribute>,
    ident: Ident,
    fields: Vec<TypeExpression>,
    options: Options,
) -> syn::Result<LoweredDeclaration> {
    let mut items = Vec::new();
    let mut lowered_fields = Vec::with_capacity(fields.len());

    for (index, field) in fields.into_iter().enumerate() {
        let lowered = lower_type(field, options)?;
        items.extend(lowered.items);
        let field_ty = lowered.ty;
        let field_documentation = LitStr::new(
            &format!("Field `{ident}::{index}` generated by `strutuct!`."),
            ident.span(),
        );
        lowered_fields.push(quote! {
            #[doc = #field_documentation]
            pub #field_ty
        });
    }

    let documentation = LitStr::new(
        &format!("Tuple struct generated by `strutuct!` for `{ident}`."),
        ident.span(),
    );
    items.push(quote! {
        #(#attrs)*
        #[doc = #documentation]
        pub struct #ident(#(#lowered_fields),*);
    });
    items.push(emit_tuple_macro(&ident));

    Ok(LoweredDeclaration {
        items,
        ident,
        kind: DeclarationKind::Tuple,
    })
}

/// Emits a public enum and a recursively delegating constructor macro.
fn lower_enum(
    attrs: Vec<Attribute>,
    ident: Ident,
    variants: Vec<ParsedEnumVariant>,
    options: Options,
) -> syn::Result<LoweredDeclaration> {
    let mut items = Vec::new();
    let mut lowered_variants = Vec::with_capacity(variants.len());
    let mut macro_arms = Vec::new();

    for variant in variants {
        let ParsedEnumVariant {
            attrs: variant_attrs,
            product_variants,
            kind,
        } = variant;
        let product_variants = product_variants.unwrap_or(options.product_variants);
        match kind {
            EnumVariant::Implicit(ty) => {
                let lowered = lower_type(*ty, options)?;
                items.extend(lowered.items);
                let payload_name = lowered.name.ok_or_else(|| {
                    Error::new(
                        ident.span(),
                        "cannot derive an implicit variant name from this type; name the variant explicitly",
                    )
                })?;
                let variant_ident = concatenate(&[&payload_name, &ident]);
                let payload_ty = lowered.ty;
                let variant_documentation = LitStr::new(
                    &format!("Variant `{ident}::{variant_ident}` generated by `strutuct!`."),
                    variant_ident.span(),
                );
                lowered_variants.push(quote! {
                    #(#variant_attrs)*
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
                        Some(DeclarationKind::Tuple) => {
                            macro_arms.push(quote! {
                                (#payload_name($($fields:tt)*)) => {
                                    #ident::#variant_ident(#payload_name!($($fields)*))
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
                let mut fields = fields
                    .into_iter()
                    .map(|field| lower_type(field, options))
                    .collect::<syn::Result<Vec<_>>>()?;
                for field in &mut fields {
                    items.append(&mut field.items);
                }
                if let [field] = fields.as_slice()
                    && !field.wrapped
                    && let (Some(payload_name), Some(kind)) = (&field.name, field.nested_kind)
                {
                    macro_arms.push(nested_variant_arm(
                        &ident,
                        &variant_ident,
                        &variant_ident,
                        payload_name,
                        kind,
                    ));
                }
                let product = product_variants && fields.len() >= 2;
                if product {
                    macro_arms.push(quote! {
                        (#variant_ident($($arguments:tt)*)) => {
                            #ident::#variant_ident(($($arguments)*))
                        };
                    });
                }
                let mut lowered_fields = Vec::with_capacity(fields.len());
                for field in fields {
                    lowered_fields.push(field.ty);
                }
                let variant_documentation = LitStr::new(
                    &format!("Variant `{ident}::{variant_ident}` generated by `strutuct!`."),
                    variant_ident.span(),
                );
                if product {
                    lowered_variants.push(quote! {
                        #(#variant_attrs)*
                        #[doc = #variant_documentation]
                        #variant_ident((#(#lowered_fields),*))
                    });
                } else {
                    lowered_variants.push(quote! {
                        #(#variant_attrs)*
                        #[doc = #variant_documentation]
                        #variant_ident(#(#lowered_fields),*)
                    });
                }
            }
            EnumVariant::Generated {
                ident: variant_ident,
                declaration,
            } => {
                let Declaration {
                    attrs: declaration_attrs,
                    ident: payload_ident,
                    body,
                } = *declaration;

                match (product_variants, body) {
                    (false, DeclarationBody::Struct(fields)) => {
                        let mut lowered_fields = Vec::with_capacity(fields.len());
                        for field in fields {
                            let lowered = lower_type(field.ty, options)?;
                            items.extend(lowered.items);
                            let field_attrs = field.attrs;
                            let field_ident = field.ident;
                            let field_ty = lowered.ty;
                            let field_documentation = LitStr::new(
                                &format!(
                                    "Field `{ident}::{variant_ident}::{field_ident}` generated by `strutuct!`."
                                ),
                                field_ident.span(),
                            );
                            lowered_fields.push(quote! {
                                #(#field_attrs)*
                                #[doc = #field_documentation]
                                #field_ident: #field_ty
                            });
                        }
                        let variant_documentation = LitStr::new(
                            &format!(
                                "Variant `{ident}::{variant_ident}` generated by `strutuct!`."
                            ),
                            variant_ident.span(),
                        );
                        lowered_variants.push(quote! {
                            #(#variant_attrs)*
                            #(#declaration_attrs)*
                            #[doc = #variant_documentation]
                            #variant_ident { #(#lowered_fields),* }
                        });
                    }
                    (_, body) => {
                        let lowered = lower_declaration(
                            Declaration {
                                attrs: declaration_attrs,
                                ident: payload_ident,
                                body,
                            },
                            options,
                        )?;
                        items.extend(lowered.items);
                        let payload_ident = lowered.ident;
                        macro_arms.push(nested_variant_arm(
                            &ident,
                            &variant_ident,
                            &variant_ident,
                            &payload_ident,
                            lowered.kind,
                        ));
                        let variant_documentation = LitStr::new(
                            &format!(
                                "Variant `{ident}::{variant_ident}` generated by `strutuct!`."
                            ),
                            variant_ident.span(),
                        );
                        lowered_variants.push(quote! {
                            #(#variant_attrs)*
                            #[doc = #variant_documentation]
                            #variant_ident(#payload_ident)
                        });
                    }
                }
            }
            EnumVariant::Unit(variant_ident) => {
                let variant_documentation = LitStr::new(
                    &format!("Variant `{ident}::{variant_ident}` generated by `strutuct!`."),
                    variant_ident.span(),
                );
                lowered_variants.push(quote! {
                    #(#variant_attrs)*
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
        #(#attrs)*
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

/// Emits one constructor arm delegating through a generated payload type.
fn nested_variant_arm(
    parent: &Ident,
    variant: &Ident,
    selector: &Ident,
    payload: &Ident,
    kind: DeclarationKind,
) -> TokenStream {
    match kind {
        DeclarationKind::Enum => quote! {
            (#selector::$($tail:tt)+) => {
                #parent::#variant(#payload!($($tail)+))
            };
        },
        DeclarationKind::Struct => quote! {
            (#selector { $($fields:tt)* }) => {
                #parent::#variant(#payload! { $($fields)* })
            };
        },
        DeclarationKind::Tuple => quote! {
            (#selector($($fields:tt)*)) => {
                #parent::#variant(#payload!($($fields)*))
            };
        },
    }
}

/// Lowers a type expression and marks wrapped recursive edges as terminal.
fn lower_type(expression: TypeExpression, options: Options) -> syn::Result<LoweredType> {
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
            let declaration = lower_declaration(*declaration, options)?;
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

/// Emits a same-name macro that constructs a generated tuple struct.
fn emit_tuple_macro(ident: &Ident) -> TokenStream {
    let documentation = LitStr::new(
        &format!("Constructs a `{ident}` value generated by `strutuct!`."),
        ident.span(),
    );
    quote! {
        #[doc = #documentation]
        #[allow(unused_macros)]
        macro_rules! #ident {
            ($($fields:tt)*) => {
                #ident($($fields)*)
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

    /// Replaces a family derive for one nested declaration and its descendants.
    #[test]
    fn local_derive_overrides_the_nested_branch() {
        let expanded = expand(
            "#[derive(Clone, Default)] Root #[derive(Debug)] branch: Branch { leaf: Leaf { value: String } },",
        );
        let derives_for = |name: &str| {
            let mut derives = expanded
                .items
                .iter()
                .find_map(|item| {
                    let (ident, attrs) = match item {
                        syn::Item::Enum(item) => (&item.ident, &item.attrs),
                        syn::Item::Struct(item) => (&item.ident, &item.attrs),
                        _ => return None,
                    };
                    (ident == name).then(|| {
                        attrs
                            .iter()
                            .filter(|attribute| attribute.path().is_ident("derive"))
                            .flat_map(|attribute| {
                                attribute
                                    .parse_args_with(
                                        Punctuated::<Path, Token![,]>::parse_terminated,
                                    )
                                    .expect("derive paths")
                            })
                            .map(|derive| derive.to_token_stream().to_string())
                            .collect::<Vec<_>>()
                    })
                })
                .expect("generated declaration");
            derives.sort();
            derives
        };

        assert_eq!(derives_for("Root"), ["Clone", "Default"]);
        assert_eq!(derives_for("Branch"), ["Debug", "Default"]);
        assert_eq!(derives_for("Leaf"), ["Debug", "Default"]);
    }

    /// Generates same-name macros that recursively construct nested enum chains.
    #[test]
    fn generates_direct_and_transitive_constructors() {
        let expanded = expand("Expr Unary { (Pref), (Post) } (Bin) LitStr(String) Null")
            .into_token_stream()
            .to_string();

        for expected in [
            "pub enum ExprUnary",
            "PrefExprUnary (Pref)",
            "PostExprUnary (Post)",
            "pub enum Expr",
            "Unary (ExprUnary)",
            "BinExpr (Bin)",
            "LitStr (String)",
            "macro_rules ! ExprUnary",
            "ExprUnary :: PrefExprUnary (Pref ($ ($ arguments) *))",
            "macro_rules ! Expr",
            "Expr :: Unary (ExprUnary ! ($ ($ tail) +))",
            "Expr :: BinExpr (Bin ($ ($ arguments) *))",
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
