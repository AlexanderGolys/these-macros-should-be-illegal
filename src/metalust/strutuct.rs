//! Nested algebraic declarations replacing one ordinary Rust item shape.

use std::collections::HashSet;

use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
    Attribute, Error, Ident, LitBool, LitStr, Path, Token, Type, Visibility,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    parse_quote, parse_quote_spanned, parse2,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Brace, Paren},
    visit_mut::VisitMut,
};

use TokenTree::{Group as GroupTT, Ident as IdentTT, Punct as PunctTT};

use crate::helpers::preprocessing::split_config_prefix;

/// One complete invocation with declaration-local lowering options.
struct Invocation {
    /// Root algebraic declaration.
    declaration: Declaration,
}

/// Optional behavior selected for one declaration family.
#[derive(Clone, Copy)]
struct Options {
    /// Whether enums generate consuming inclusion functions for their constructor paths.
    inclusions: bool,
    /// Whether non-unit enum variants carry one explicit product value.
    product_variants: bool,
    /// Whether generated declarations and fields are public by default.
    public: bool,
    /// Whether every existing identifier concatenation is emitted in reverse order.
    reverse_concat: bool,
}

/// Options explicitly overridden on one declaration, field, or variant branch.
#[derive(Clone, Copy, Default)]
struct OptionOverrides {
    /// Per-object override for generated enum inclusion functions.
    inclusions: Option<bool>,
    /// Per-object override for unary product variants.
    product_variants: Option<bool>,
    /// Per-object override for default public visibility.
    public: Option<bool>,
    /// Per-object override for identifier concatenation order.
    reverse_concat: Option<bool>,
}

/// Supplies the default algebraic lowering behavior.
impl Default for Options {
    /// Enables unary product variants unless the declaration opts out.
    fn default() -> Self {
        Self {
            inclusions: false,
            product_variants: true,
            public: true,
            reverse_concat: false,
        }
    }
}

/// Applies local overrides while retaining inherited family options.
impl Options {
    /// Returns a copy updated by every option explicitly set on one object.
    fn with(self, overrides: OptionOverrides) -> Self {
        Self {
            inclusions: overrides.inclusions.unwrap_or(self.inclusions),
            product_variants: overrides.product_variants.unwrap_or(self.product_variants),
            public: overrides.public.unwrap_or(self.public),
            reverse_concat: overrides.reverse_concat.unwrap_or(self.reverse_concat),
        }
    }
}

impl OptionOverrides {
    /// Combines inherited and local overrides, preferring the local values.
    fn with(self, local: Self) -> Self {
        Self {
            inclusions: local.inclusions.or(self.inclusions),
            product_variants: local.product_variants.or(self.product_variants),
            public: local.public.or(self.public),
            reverse_concat: local.reverse_concat.or(self.reverse_concat),
        }
    }
}

/// One declaration inferred to be either a struct or an enum from its members.
struct Declaration {
    /// Ordinary Rust attributes forwarded to the generated declaration.
    attrs: Vec<Attribute>,
    /// Configuration inherited by this declaration and its nested branch.
    options: OptionOverrides,
    /// Explicit Rust visibility, with inherited visibility representing `priv`.
    visibility: Option<Visibility>,
    /// Name of the generated Rust type.
    ident: Ident,
    /// Whether this name is exact or relative to its generated parent.
    name: DeclarationName,
    /// Struct fields or enum variants belonging to the declaration.
    body: DeclarationBody,
}

/// How an inline declaration's identifier becomes its generated Rust name.
#[derive(Clone, Copy)]
enum DeclarationName {
    /// Keeps the identifier exactly as written, as for roots and `|Type|`.
    Exact,
    /// Concatenates the identifier with its generated parent declaration.
    Relative,
}

/// The three declaration shapes inferred from the DSL body.
enum DeclarationBody {
    /// A nominal unit type declared without a body.
    Unit,
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
    /// Configuration applied to this field and declarations nested in its type.
    options: OptionOverrides,
    /// Explicit field visibility, with inherited visibility representing `priv`.
    visibility: Option<Visibility>,
    /// Name retained for the generated Rust field.
    ident: Ident,
    /// Possibly nested or postfix-wrapped field type.
    ty: TypeExpression,
}

/// One parsed enum variant together with attributes and a local behavior override.
struct ParsedEnumVariant {
    /// Ordinary Rust attributes forwarded to the generated variant.
    attrs: Vec<Attribute>,
    /// Configuration applied to this variant and its generated payload branch.
    options: OptionOverrides,
    /// Visibility applied to an inline payload declaration owned by this variant.
    visibility: Option<Visibility>,
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

/// One braced declaration parsed from a reconstructed type-token prefix.
struct InlineDeclaration {
    /// Parsed declaration returned to the containing type expression.
    declaration: Declaration,
    /// Tokens following the inline declaration in its containing Rust type.
    remaining: TokenStream,
}

/// Whether a Rust-type suffix starts with the extended declaration grammar.
struct InlineDeclarationProbe {
    /// Result of checking the suffix before consuming it as unrestricted tokens.
    begins: bool,
}

/// The unwrapped portion of a field or variant type.
enum TypeBase {
    /// A Rust type together with declarations embedded in its generic arguments.
    Rust {
        /// The ordinary Rust type with inline declarations replaced by unique placeholders.
        ty: Box<Type>,
        /// Inline declarations hoisted out of the Rust type.
        nested: Vec<NestedDeclaration>,
    },
    /// A nested declaration that must be hoisted before its parent.
    Nested(Box<Declaration>),
}

/// One inline declaration and its collision-free placeholder inside a parsed Rust type.
struct NestedDeclaration {
    /// Temporary identifier replaced by the declaration's final generated name.
    placeholder: Ident,
    /// Declaration hoisted out of its surrounding Rust type.
    declaration: Declaration,
}

/// Replaces one internal type placeholder with its resolved generated identifier.
struct ReplacePlaceholder<'a> {
    /// Placeholder inserted before the surrounding Rust type was parsed.
    placeholder: &'a Ident,
    /// Final bottom-up name of the generated declaration.
    replacement: &'a Ident,
}

impl VisitMut for ReplacePlaceholder<'_> {
    /// Rewrites the unique placeholder wherever it occurs in the parsed type.
    fn visit_ident_mut(&mut self, ident: &mut Ident) {
        if ident == self.placeholder {
            *ident = self.replacement.clone();
        }
    }
}

/// Supported postfix type constructors.
#[derive(Clone, Copy)]
enum TypeWrapper {
    /// `T?`, lowered to `Option<T>`.
    Option,
    /// `T*`, lowered to `Box<T>`.
    Box,
}

/// Whether a lowered declaration is a product or sum type.
#[derive(Clone, Copy)]
enum DeclarationKind {
    /// A generated unit struct whose value is its name.
    Unit,
    /// A generated struct whose constructor macro accepts fields.
    Struct,
    /// A generated tuple struct whose constructor macro accepts positional fields.
    Tuple,
    /// A generated enum whose constructor macro selects a variant path.
    Enum,
}

/// Optional Rust-like keyword used only to validate an inferred declaration shape.
#[derive(Clone, Copy)]
enum DeclarationKeyword {
    /// Requires an inferred unit, named-field, or tuple struct.
    Struct(Span),
    /// Requires an inferred enum.
    Enum(Span),
}

/// Fully lowered declaration together with its generated items.
struct LoweredDeclaration {
    /// Generated items ordered with dependencies before their consumers.
    items: Vec<TokenStream>,
    /// Name usable when the declaration is embedded as a type.
    ident: Ident,
    /// Reusable algebraic description of the generated Rust type.
    algebra: AlgebraicType,
}

/// A lowered Rust type retaining the algebraic structure relevant to generated maps.
struct AlgebraicType {
    /// Ordinary Rust spelling of this type.
    ty: TokenStream,
    /// Unit, product, coproduct, wrapper, or opaque-container structure.
    shape: AlgebraicShape,
    /// Conditions governing the availability of this complete type.
    conditional_attrs: Vec<Attribute>,
    /// Tuple or unit constructor occupying the type's value namespace.
    value_constructor: Option<Ident>,
    /// Constructor-macro behavior of a declaration generated by `strutuct!`.
    declaration_kind: Option<DeclarationKind>,
}

/// Structural algebraic form retained after the Rust items have been lowered.
enum AlgebraicShape {
    /// An ordinary Rust type, optionally containing independently generated declarations.
    Rust(Vec<AlgebraicType>),
    /// The terminal one-element product.
    Unit,
    /// A product with named or positional components.
    Product(Vec<ProductComponent>),
    /// A coproduct with one constructor for every variant.
    Coproduct(Vec<CoproductVariant>),
    /// A postfix ownership wrapper around another algebraic type.
    Wrapped {
        /// Wrapper applied at this level.
        wrapper: TypeWrapper,
        /// Algebraic type enclosed by the wrapper.
        inner: Box<AlgebraicType>,
    },
}

/// One algebraic layer whose immediate children have already been folded.
enum FoldedAlgebraicShape<'a, T> {
    /// Ordinary Rust syntax with independently generated declarations inside it.
    Rust(Vec<T>),
    /// The terminal one-element product.
    Unit,
    /// Product selectors paired with their folded component types.
    Product(Vec<(&'a ProductSelector, T)>),
    /// Coproduct branches paired with their folded payload types.
    Coproduct(Vec<(&'a CoproductVariant, T)>),
    /// One ownership wrapper around its folded inner type.
    Wrapped {
        /// Wrapper applied at this layer.
        wrapper: TypeWrapper,
        /// Folded value of the enclosed type.
        inner: T,
    },
}

/// One component of a named or positional product.
struct ProductComponent {
    /// Field name or positional index selecting this component.
    selector: ProductSelector,
    /// Algebraic type stored in this component.
    ty: AlgebraicType,
}

/// Projection label belonging to one product component.
enum ProductSelector {
    /// Named-field projection such as `.value`.
    Named(Ident),
    /// Positional projection such as `.0`.
    Positional(usize),
}

impl ProductSelector {
    /// Returns the ordinary Rust projection spelling for this product component.
    fn spelling(&self) -> String {
        match self {
            Self::Named(ident) => format!(".{ident}"),
            Self::Positional(index) => format!(".{index}"),
        }
    }
}

/// One branch of a coproduct together with its concrete Rust constructor.
struct CoproductVariant {
    /// Selector used by the `strutuct!` constructor path.
    selector: Ident,
    /// Actual Rust variant name emitted in the enum declaration.
    variant: Ident,
    /// Algebraic payload injected by this branch.
    payload: AlgebraicType,
    /// How the payload is passed to the concrete Rust variant.
    constructor: CoproductConstructor,
    /// Conditions governing this variant in addition to its payload type.
    conditional_attrs: Vec<Attribute>,
    /// Whether inclusion generation follows this edge.
    inclusions: bool,
    /// Concatenation policy active on this edge.
    reverse_concat: bool,
}

/// Concrete Rust constructor shape implementing one coproduct injection.
enum CoproductConstructor {
    /// A unit variant written without parentheses.
    Unit,
    /// A variant taking exactly one payload value.
    Unary,
    /// A Rust multi-field tuple variant fed from one tuple product.
    Tuple(usize),
    /// A Rust struct-like variant fed from one positional product.
    Struct(Vec<Ident>),
}

/// One constructor occurrence from a payload into the enum currently being lowered.
struct InclusionPath {
    /// Variant selectors appended to the owning enum name.
    steps: Vec<InclusionStep>,
    /// Optional consumed value type; absence represents a unit variant.
    parameter: Option<TokenStream>,
    /// Expression constructing the current enum from the function argument.
    value: TokenStream,
    /// Conditions required for every type and variant along the path.
    conditional_attrs: Vec<Attribute>,
    /// Tuple or unit constructor that may collide with the generated function name.
    value_constructor: Option<Ident>,
}

impl AlgebraicType {
    /// Folds the complete algebraic tree bottom-up, retaining labels on every edge.
    fn fold<T, F>(&self, algebra: &mut F) -> T
    where
        F: for<'a> FnMut(&'a Self, FoldedAlgebraicShape<'a, T>) -> T,
    {
        let folded = match &self.shape {
            AlgebraicShape::Rust(nested) => {
                FoldedAlgebraicShape::Rust(nested.iter().map(|ty| ty.fold(algebra)).collect())
            }
            AlgebraicShape::Unit => FoldedAlgebraicShape::Unit,
            AlgebraicShape::Product(components) => FoldedAlgebraicShape::Product(
                components
                    .iter()
                    .map(|component| (&component.selector, component.ty.fold(algebra)))
                    .collect(),
            ),
            AlgebraicShape::Coproduct(variants) => FoldedAlgebraicShape::Coproduct(
                variants
                    .iter()
                    .map(|variant| (variant, variant.payload.fold(algebra)))
                    .collect(),
            ),
            AlgebraicShape::Wrapped { wrapper, inner } => FoldedAlgebraicShape::Wrapped {
                wrapper: *wrapper,
                inner: inner.fold(algebra),
            },
        };
        algebra(self, folded)
    }

    /// Counts this node and every algebraic type structurally contained below it.
    fn node_count(&self) -> usize {
        self.fold(&mut |_, folded| match folded {
            FoldedAlgebraicShape::Rust(nested) => 1 + nested.into_iter().sum::<usize>(),
            FoldedAlgebraicShape::Unit => 1,
            FoldedAlgebraicShape::Product(components) => {
                1 + components
                    .into_iter()
                    .map(|(selector, size)| {
                        let _ = selector.spelling();
                        size
                    })
                    .sum::<usize>()
            }
            FoldedAlgebraicShape::Coproduct(variants) => {
                1 + variants
                    .into_iter()
                    .map(|(variant, size)| {
                        let _ = variant;
                        size
                    })
                    .sum::<usize>()
            }
            FoldedAlgebraicShape::Wrapped { wrapper, inner } => {
                let _ = wrapper;
                1 + inner
            }
        })
    }

    /// Derives root injections by iteratively joining homogeneous coproduct layers.
    fn flattened_coproduct(&self) -> Vec<InclusionPath> {
        self.fold(
            &mut |node, folded: FoldedAlgebraicShape<'_, Vec<InclusionPath>>| {
                let FoldedAlgebraicShape::Coproduct(variants) = folded else {
                    return Vec::new();
                };
                let mut flattened = Vec::new();

                for (branch, nested_paths) in
                    variants.into_iter().filter(|(branch, _)| branch.inclusions)
                {
                    let step = InclusionStep {
                        selector: branch.selector.clone(),
                        reverse_concat: branch.reverse_concat,
                    };
                    let branch_attrs = combined_conditional_attrs(
                        &node.conditional_attrs,
                        &branch.conditional_attrs,
                    );
                    let direct_attrs = combined_conditional_attrs(
                        &branch_attrs,
                        &branch.payload.conditional_attrs,
                    );
                    let value = inclusion_value_ident(branch.variant.span());
                    flattened.push(InclusionPath {
                        steps: vec![InclusionStep {
                            selector: step.selector.clone(),
                            reverse_concat: step.reverse_concat,
                        }],
                        parameter: branch
                            .constructor
                            .takes_value()
                            .then(|| branch.payload.ty.clone()),
                        value: branch.constructor.construct(
                            &node.ty,
                            &branch.variant,
                            Some(value.to_token_stream()),
                        ),
                        conditional_attrs: direct_attrs,
                        value_constructor: branch.payload.value_constructor.clone(),
                    });

                    if !branch.constructor.composes() {
                        continue;
                    }
                    for mut nested in nested_paths {
                        nested.steps.insert(
                            0,
                            InclusionStep {
                                selector: step.selector.clone(),
                                reverse_concat: step.reverse_concat,
                            },
                        );
                        nested.value = branch.constructor.construct(
                            &node.ty,
                            &branch.variant,
                            Some(nested.value),
                        );
                        nested.conditional_attrs =
                            combined_conditional_attrs(&branch_attrs, &nested.conditional_attrs);
                        flattened.push(nested);
                    }
                }

                flattened
            },
        )
    }
}

impl CoproductConstructor {
    /// Reports whether the concrete variant constructor consumes one product value.
    fn takes_value(&self) -> bool {
        !matches!(self, Self::Unit | Self::Tuple(0))
    }

    /// Reports whether a nested coproduct remains composable through this constructor.
    fn composes(&self) -> bool {
        matches!(self, Self::Unary)
    }

    /// Applies this concrete Rust variant constructor to an optional product expression.
    fn construct(
        &self,
        parent: &TokenStream,
        variant: &Ident,
        value: Option<TokenStream>,
    ) -> TokenStream {
        match self {
            Self::Unit => quote!(#parent::#variant),
            Self::Unary => {
                let value = value.expect("unary coproduct constructors have a payload");
                quote!(#parent::#variant(#value))
            }
            Self::Tuple(arity) => {
                if *arity == 0 {
                    return quote!(#parent::#variant());
                }
                let value = value.expect("nonempty tuple coproduct constructors have a payload");
                let fields = (0..*arity).map(syn::Index::from);
                quote!(#parent::#variant(#(#value.#fields),*))
            }
            Self::Struct(fields) => {
                let value = value.expect("struct coproduct constructors have a payload");
                let indices = (0..fields.len()).map(syn::Index::from);
                quote!(#parent::#variant { #(#fields: #value.#indices),* })
            }
        }
    }
}

/// One named enum edge and the concatenation policy active on that edge.
struct InclusionStep {
    /// Selector written for this variant in the `strutuct!` input.
    selector: Ident,
    /// Whether this edge prepends its selector to the accumulated path name.
    reverse_concat: bool,
}

/// Fully lowered type expression and nested items collected from it.
struct LoweredType {
    /// Nested declarations and constructor macros emitted before the parent.
    items: Vec<TokenStream>,
    /// Ordinary Rust type replacing the extended input spelling.
    ty: TokenStream,
    /// Identifier used to derive an implicit variant name, when one exists.
    name: Option<Ident>,
    /// Reusable algebraic description of the lowered type.
    algebra: AlgebraicType,
    /// Whether postfix syntax prevents direct recursive construction.
    wrapped: bool,
}

/// Parses one complete `strutuct!` invocation.
impl Parse for Invocation {
    /// Parses declaration attributes, options, the root name, and its body.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = input.call(Attribute::parse_outer)?;
        let invocation_options = take_options(&mut attrs)?;
        let options = if input.peek(Token![;]) {
            input.parse::<Token![;]>()?;
            let mut declaration_attrs = input.call(Attribute::parse_outer)?;
            let declaration_options = take_options(&mut declaration_attrs)?;
            attrs.extend(declaration_attrs);
            invocation_options.with(declaration_options)
        } else {
            invocation_options
        };
        let visibility = parse_visibility(input)?;
        let keyword = parse_declaration_keyword(input)?;
        let ident = Ident::parse_any(input)?;
        let declaration = if input.peek(Brace) {
            let content;
            syn::braced!(content in input);
            if !input.is_empty() {
                return Err(input.error("unexpected tokens after braced root declaration"));
            }
            parse_declaration(
                attrs,
                options,
                visibility,
                keyword,
                ident,
                DeclarationName::Exact,
                &content,
            )?
        } else {
            parse_declaration(
                attrs,
                options,
                visibility,
                keyword,
                ident,
                DeclarationName::Exact,
                input,
            )?
        };
        Ok(Self { declaration })
    }
}

/// Parses ordinary and nested types followed by `?` and `*` operators.
impl Parse for TypeExpression {
    /// Parses the base type before consuming left-associative postfix wrappers.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (base, wrappers) = if begins_inline_declaration(input) {
            (
                TypeBase::Nested(Box::new(parse_inline_declaration(input)?)),
                parse_type_wrappers(input)?,
            )
        } else {
            parse_rust_type(input)?
        };

        Ok(Self { base, wrappers })
    }
}

/// Parses an identifier followed by its complete inline declaration body.
impl Parse for InlineDeclaration {
    /// Parses one generated declaration and retains the containing type suffix.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            declaration: parse_inline_declaration(input)?,
            remaining: input.parse()?,
        })
    }
}

/// Probes a complete suffix while leaving semantic parsing to `InlineDeclaration`.
impl Parse for InlineDeclarationProbe {
    /// Records whether the suffix begins with a declaration, then consumes the suffix.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let begins = begins_inline_declaration(input);
        let _: TokenStream = input.parse()?;
        Ok(Self { begins })
    }
}

/// Parses explicit Rust visibility or the DSL's private `priv` marker.
fn parse_visibility(input: ParseStream) -> syn::Result<Option<Visibility>> {
    let fork = input.fork();
    if let Ok(identifier) = Ident::parse_any(&fork)
        && identifier == "priv"
    {
        Ident::parse_any(input)?;
        return Ok(Some(Visibility::Inherited));
    }

    let visibility: Visibility = input.parse()?;
    Ok((!matches!(visibility, Visibility::Inherited)).then_some(visibility))
}

/// Consumes an optional decorative `struct` or `enum` keyword.
fn parse_declaration_keyword(input: ParseStream) -> syn::Result<Option<DeclarationKeyword>> {
    if input.peek(Token![struct]) {
        let token = input.parse::<Token![struct]>()?;
        Ok(Some(DeclarationKeyword::Struct(token.span)))
    } else if input.peek(Token![enum]) {
        let token = input.parse::<Token![enum]>()?;
        Ok(Some(DeclarationKeyword::Enum(token.span)))
    } else {
        Ok(None)
    }
}

/// Reports whether the next tokens describe an inline generated declaration.
fn begins_inline_declaration(input: ParseStream) -> bool {
    let fork = input.fork();
    let Ok(mut attrs) = fork.call(Attribute::parse_outer) else {
        return false;
    };
    if take_options(&mut attrs).is_err()
        || parse_visibility(&fork).is_err()
        || parse_declaration_keyword(&fork).is_err()
    {
        return false;
    }
    if fork.peek(Token![|]) {
        return true;
    }
    Ident::parse_any(&fork).is_ok() && fork.peek(Brace)
}

/// Parses one inline declaration with its own attributes, options, and visibility.
fn parse_inline_declaration(input: ParseStream) -> syn::Result<Declaration> {
    let mut attrs = input.call(Attribute::parse_outer)?;
    let options = take_options(&mut attrs)?;
    let visibility = parse_visibility(input)?;
    let keyword = parse_declaration_keyword(input)?;

    if input.peek(Token![|]) {
        return parse_named_declaration(input, attrs, options, visibility, keyword);
    }

    let ident = Ident::parse_any(input)?;
    let content;
    syn::braced!(content in input);
    parse_declaration(
        attrs,
        options,
        visibility,
        keyword,
        ident,
        DeclarationName::Relative,
        &content,
    )
}

/// Parses postfix ownership wrappers that follow a directly nested declaration.
fn parse_type_wrappers(input: ParseStream) -> syn::Result<Vec<TypeWrapper>> {
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
    Ok(wrappers)
}

/// Parses one ordinary Rust type while hoisting declarations from inside it.
fn parse_rust_type(input: ParseStream) -> syn::Result<(TypeBase, Vec<TypeWrapper>)> {
    let mut tokens = Vec::new();
    let mut angle_depth = 0usize;
    while !(input.is_empty() || angle_depth == 0 && input.peek(Token![,])) {
        let token = input.parse::<TokenTree>()?;
        if let PunctTT(punctuation) = &token {
            match punctuation.as_char() {
                '<' => angle_depth += 1,
                '>' if angle_depth > 0 => angle_depth -= 1,
                _ => {}
            }
        }
        tokens.push(token);
    }

    let mut wrappers = Vec::new();
    while let Some(PunctTT(punctuation)) = tokens.last() {
        let wrapper = match punctuation.as_char() {
            '?' => TypeWrapper::Option,
            '*' => TypeWrapper::Box,
            _ => break,
        };
        tokens.pop();
        wrappers.push(wrapper);
    }
    wrappers.reverse();

    let (tokens, nested) = rewrite_nested_type_declarations(tokens)?;
    let ty = parse2::<Type>(tokens)?;
    Ok((
        TypeBase::Rust {
            ty: Box::new(ty),
            nested,
        },
        wrappers,
    ))
}

/// Replaces inline declarations by their names throughout one Rust type.
fn rewrite_nested_type_declarations(
    tokens: Vec<TokenTree>,
) -> syn::Result<(TokenStream, Vec<NestedDeclaration>)> {
    let mut identifiers = HashSet::new();
    collect_identifiers(&tokens, &mut identifiers);
    let mut next_placeholder = 0;
    rewrite_nested_type_declarations_inner(tokens, &mut identifiers, &mut next_placeholder)
}

/// Recursively collects identifiers so internal placeholders cannot shadow user input.
fn collect_identifiers(tokens: &[TokenTree], identifiers: &mut HashSet<String>) {
    for token in tokens {
        match token {
            IdentTT(ident) => {
                identifiers.insert(ident.to_string());
            }
            GroupTT(group) => {
                collect_identifiers(&group.stream().into_iter().collect::<Vec<_>>(), identifiers);
            }
            _ => {}
        }
    }
}

/// Rewrites inline declarations while sharing placeholder state across nested groups.
fn rewrite_nested_type_declarations_inner(
    tokens: Vec<TokenTree>,
    identifiers: &mut HashSet<String>,
    next_placeholder: &mut usize,
) -> syn::Result<(TokenStream, Vec<NestedDeclaration>)> {
    let mut output = TokenStream::new();
    let mut declarations = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if let Some((length, declaration)) = parse_inline_type_declaration(&tokens[index..])? {
            let placeholder = loop {
                let spelling = format!("__strutuct_inline_declaration_{next_placeholder}");
                *next_placeholder += 1;
                if identifiers.insert(spelling.clone()) {
                    break Ident::new(&spelling, declaration.ident.span());
                }
            };
            output.extend([IdentTT(placeholder.clone())]);
            declarations.push(NestedDeclaration {
                placeholder,
                declaration,
            });
            index += length;
            continue;
        }

        if let Some(length) = opaque_type_prefix_length(&tokens[index..]) {
            output.extend(tokens[index..index + length].iter().cloned());
            index += length;
            continue;
        }

        match tokens[index].clone() {
            GroupTT(group) => {
                let (stream, mut nested) = rewrite_nested_type_declarations_inner(
                    group.stream().into_iter().collect(),
                    identifiers,
                    next_placeholder,
                )?;
                let mut rewritten = Group::new(group.delimiter(), stream);
                rewritten.set_span(group.span());
                output.extend([GroupTT(rewritten)]);
                declarations.append(&mut nested);
            }
            token => output.extend([token]),
        }
        index += 1;
    }

    Ok((output, declarations))
}

/// Reports an attribute or macro invocation whose contents are opaque type tokens.
fn opaque_type_prefix_length(tokens: &[TokenTree]) -> Option<usize> {
    if matches!(tokens.first(), Some(PunctTT(punctuation)) if punctuation.as_char() == '#') {
        if matches!(tokens.get(1), Some(GroupTT(group)) if group.delimiter() == proc_macro2::Delimiter::Bracket)
        {
            return Some(2);
        }
        if matches!(tokens.get(1), Some(PunctTT(punctuation)) if punctuation.as_char() == '!')
            && matches!(tokens.get(2), Some(GroupTT(group)) if group.delimiter() == proc_macro2::Delimiter::Bracket)
        {
            return Some(3);
        }
    }

    if matches!(tokens.first(), Some(IdentTT(_)))
        && matches!(tokens.get(1), Some(PunctTT(punctuation)) if punctuation.as_char() == '!')
        && matches!(tokens.get(2), Some(GroupTT(_)))
    {
        return Some(3);
    }

    None
}

/// Parses an inline declaration prefix used as one component of a Rust type.
fn parse_inline_type_declaration(
    tokens: &[TokenTree],
) -> syn::Result<Option<(usize, Declaration)>> {
    let input: TokenStream = tokens.iter().cloned().collect();
    if !parse2::<InlineDeclarationProbe>(input.clone())?.begins {
        return Ok(None);
    }
    let parsed = parse2::<InlineDeclaration>(input)?;
    let consumed = tokens.len() - parsed.remaining.into_iter().count();
    Ok(Some((consumed, parsed.declaration)))
}

/// Expands one nested algebraic declaration into ordinary Rust items.
pub(crate) fn strutuct(input: TokenStream) -> TokenStream {
    let result = split_config_prefix(input)
        .and_then(|(_, input)| parse2::<Invocation>(input))
        .and_then(|mut invocation| {
            merge_declaration_attrs(&mut invocation.declaration, &[])?;
            lower_declaration(invocation.declaration, Options::default(), false)
        })
        .map(|declaration| declaration.items.into_iter().collect());

    result.unwrap_or_else(Error::into_compile_error)
}

/// Selects root attributes that generated nested declarations must also carry.
fn propagated_declaration_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|attribute| !is_doc(attribute) && !is_underive(attribute))
        .cloned()
        .collect()
}

/// Reports whether an attribute is documentation local to one generated item.
fn is_doc(attribute: &Attribute) -> bool {
    attribute.path().is_ident("doc")
}

/// Reports whether an attribute declares one or more derived traits.
fn is_derive(attribute: &Attribute) -> bool {
    attribute.path().is_ident("derive")
}

/// Reports whether an attribute removes traits from one generated branch.
fn is_underive(attribute: &Attribute) -> bool {
    attribute.path().is_ident("underive")
}

/// Retains attributes that determine whether a generated item exists.
fn conditional_compilation_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|attribute| {
            attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
        })
        .cloned()
        .collect()
}

/// Applies inherited declaration attributes to every nested generated type.
fn propagate_declaration_attrs(body: &mut DeclarationBody, attrs: &[Attribute]) -> syn::Result<()> {
    match body {
        DeclarationBody::Unit => {}
        DeclarationBody::Struct(fields) => {
            for field in fields {
                propagate_type_attrs(&mut field.ty, attrs)?;
            }
        }
        DeclarationBody::Tuple(fields) => {
            for field in fields {
                propagate_type_attrs(field, attrs)?;
            }
        }
        DeclarationBody::Enum(variants) => {
            for variant in variants {
                match &mut variant.kind {
                    EnumVariant::Implicit(ty) => propagate_type_attrs(ty, attrs)?,
                    EnumVariant::Tuple { fields, .. } => {
                        for field in fields {
                            propagate_type_attrs(field, attrs)?;
                        }
                    }
                    EnumVariant::Generated { declaration, .. } => {
                        merge_declaration_attrs(declaration, attrs)?;
                    }
                    EnumVariant::Unit(_) => {}
                }
            }
        }
    }

    Ok(())
}

/// Applies inherited attributes when a type expression defines a nested type.
fn propagate_type_attrs(expression: &mut TypeExpression, attrs: &[Attribute]) -> syn::Result<()> {
    match &mut expression.base {
        TypeBase::Rust { nested, .. } => {
            for nested in nested {
                merge_declaration_attrs(&mut nested.declaration, attrs)?;
            }
        }
        TypeBase::Nested(declaration) => {
            merge_declaration_attrs(declaration, attrs)?;
        }
    }

    Ok(())
}

/// Parses the trait paths from one derive attribute.
fn parse_derive_paths(attribute: &Attribute) -> syn::Result<Vec<Path>> {
    attribute
        .parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
        .map(|paths| paths.into_iter().collect())
}

/// Adds paths that are not already present in a derive list.
fn extend_unique_paths(paths: &mut Vec<Path>, additions: impl IntoIterator<Item = Path>) {
    for addition in additions {
        let spelling = addition.to_token_stream().to_string();
        if !paths
            .iter()
            .any(|path| path.to_token_stream().to_string() == spelling)
        {
            paths.push(addition);
        }
    }
}

/// Removes every requested trait path while ignoring paths that are absent.
fn remove_paths(paths: &mut Vec<Path>, removals: impl IntoIterator<Item = Path>) {
    let removals = removals
        .into_iter()
        .map(|path| path.to_token_stream().to_string())
        .collect::<Vec<_>>();
    paths.retain(|path| !removals.contains(&path.to_token_stream().to_string()));
}

/// Merges inherited attributes into one declaration and recursively visits its body.
fn merge_declaration_attrs(declaration: &mut Declaration, attrs: &[Attribute]) -> syn::Result<()> {
    let derive_span = declaration
        .attrs
        .iter()
        .chain(attrs)
        .find(|attribute| is_derive(attribute))
        .map_or_else(Span::call_site, Attribute::span);
    let mut derives = Vec::new();
    for attribute in attrs
        .iter()
        .chain(&declaration.attrs)
        .filter(|attribute| is_derive(attribute))
    {
        extend_unique_paths(&mut derives, parse_derive_paths(attribute)?);
    }
    for attribute in declaration
        .attrs
        .iter()
        .filter(|attribute| is_underive(attribute))
    {
        remove_paths(&mut derives, parse_derive_paths(attribute)?);
    }

    declaration
        .attrs
        .retain(|attribute| !is_derive(attribute) && !is_underive(attribute));
    if !derives.is_empty() {
        declaration.attrs.insert(
            0,
            parse_quote_spanned!(derive_span=> #[derive(#(#derives),*)]),
        );
    }

    for attribute in attrs
        .iter()
        .filter(|attribute| !is_derive(attribute) && !is_underive(attribute) && !is_doc(attribute))
        .rev()
    {
        if !declaration.attrs.iter().any(|existing| {
            existing.to_token_stream().to_string() == attribute.to_token_stream().to_string()
        }) {
            declaration.attrs.insert(0, attribute.clone());
        }
    }

    let descendants = propagated_declaration_attrs(&declaration.attrs);
    propagate_declaration_attrs(&mut declaration.body, &descendants)
}

/// Consumes `strutuct` configuration attributes and retains ordinary Rust attributes.
fn take_options(attrs: &mut Vec<Attribute>) -> syn::Result<OptionOverrides> {
    let mut options = OptionOverrides::default();
    let mut retained = Vec::with_capacity(attrs.len());

    for attribute in attrs.drain(..) {
        if !attribute.path().is_ident("strutuct") {
            retained.push(attribute);
            continue;
        }
        attribute.parse_nested_meta(|meta| {
            let slot = if meta.path.is_ident("inclusions") {
                &mut options.inclusions
            } else if meta.path.is_ident("product_variants") {
                &mut options.product_variants
            } else if meta.path.is_ident("public") {
                &mut options.public
            } else if meta.path.is_ident("reverse_concat") {
                &mut options.reverse_concat
            } else {
                return Err(meta.error("unknown `strutuct` option"));
            };
            if slot.is_some() {
                return Err(meta.error("duplicate `strutuct` option"));
            }
            *slot = Some(meta.value()?.parse::<LitBool>()?.value);
            Ok(())
        })?;
    }

    *attrs = retained;
    Ok(options)
}

/// Parses a declaration body after its name and infers its algebraic shape.
fn parse_declaration(
    attrs: Vec<Attribute>,
    options: OptionOverrides,
    visibility: Option<Visibility>,
    keyword: Option<DeclarationKeyword>,
    ident: Ident,
    name: DeclarationName,
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
        DeclarationBody::Enum(parse_enum_variants(input)?)
    };
    validate_declaration_keyword(keyword, &body)?;

    Ok(Declaration {
        attrs,
        options,
        visibility,
        ident,
        name,
        body,
    })
}

/// Checks a decorative keyword against the independently inferred declaration shape.
fn validate_declaration_keyword(
    keyword: Option<DeclarationKeyword>,
    body: &DeclarationBody,
) -> syn::Result<()> {
    let Some(keyword) = keyword else {
        return Ok(());
    };
    let matches = matches!(
        (keyword, body),
        (
            DeclarationKeyword::Struct(_),
            DeclarationBody::Unit | DeclarationBody::Struct(_) | DeclarationBody::Tuple(_)
        ) | (DeclarationKeyword::Enum(_), DeclarationBody::Enum(_))
    );
    if matches {
        return Ok(());
    }

    let (span, expected, inferred) = match (keyword, body) {
        (DeclarationKeyword::Struct(span), DeclarationBody::Enum(_)) => (span, "struct", "enum"),
        (DeclarationKeyword::Enum(span), _) => (span, "enum", "struct"),
        _ => return Ok(()),
    };
    Err(Error::new(
        span,
        format!("`{expected}` does not match the inferred {inferred} declaration"),
    ))
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
    if parse_visibility(&fork).is_err() {
        return false;
    }
    Ident::parse_any(&fork).is_ok() && fork.peek(Token![:])
}

/// Parses comma-separated fields and rejects enum-shaped members in a struct.
fn parse_struct_fields(input: ParseStream) -> syn::Result<Vec<StructField>> {
    let mut fields = Vec::new();

    while !input.is_empty() {
        if !begins_struct_field(input) {
            return Err(input.error("expected a struct field in the form `name: Type`"));
        }

        let mut attrs = input.call(Attribute::parse_outer)?;
        let options = take_options(&mut attrs)?;
        let visibility = parse_visibility(input)?;
        let ident = Ident::parse_any(input)?;
        input.parse::<Token![:]>()?;
        let mut ty: TypeExpression = input.parse()?;
        let mut nested = nested_declarations_mut(&mut ty);
        if !nested.is_empty() {
            let mut field_attrs = Vec::with_capacity(attrs.len());
            for attribute in attrs {
                if is_derive(&attribute) || is_underive(&attribute) {
                    for declaration in &mut *nested {
                        declaration.attrs.push(attribute.clone());
                    }
                } else {
                    field_attrs.push(attribute);
                }
            }
            attrs = field_attrs;
        } else if let Some(modifier) = attrs
            .iter()
            .find(|attribute| is_derive(attribute) || is_underive(attribute))
        {
            return Err(Error::new_spanned(
                modifier,
                "a local derive modifier requires an inline generated field type",
            ));
        }
        fields.push(StructField {
            attrs,
            options,
            visibility,
            ident,
            ty,
        });

        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }
    }

    Ok(fields)
}

/// Returns every declaration generated anywhere inside one field type.
fn nested_declarations_mut(expression: &mut TypeExpression) -> Vec<&mut Declaration> {
    match &mut expression.base {
        TypeBase::Rust { nested, .. } => nested
            .iter_mut()
            .map(|nested| &mut nested.declaration)
            .collect(),
        TypeBase::Nested(declaration) => vec![declaration],
    }
}

/// Applies an enclosing explicit visibility to directly generated type declarations.
fn apply_visibility_to_nested_type(
    expression: &mut TypeExpression,
    visibility: Option<Visibility>,
) {
    let Some(visibility) = visibility else {
        return;
    };
    for declaration in nested_declarations_mut(expression) {
        if declaration.visibility.is_none() {
            declaration.visibility = Some(visibility.clone());
        }
    }
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

/// Parses a generated type whose explicit name appears between vertical bars.
fn parse_named_declaration(
    input: ParseStream,
    attrs: Vec<Attribute>,
    options: OptionOverrides,
    visibility: Option<Visibility>,
    keyword: Option<DeclarationKeyword>,
) -> syn::Result<Declaration> {
    input.parse::<Token![|]>()?;
    let ident = Ident::parse_any(input)?;
    input.parse::<Token![|]>()?;

    if input.peek(Brace) {
        let body;
        syn::braced!(body in input);
        parse_declaration(
            attrs,
            options,
            visibility,
            keyword,
            ident,
            DeclarationName::Exact,
            &body,
        )
    } else {
        validate_declaration_keyword(keyword, &DeclarationBody::Unit)?;
        Ok(Declaration {
            attrs,
            options,
            visibility,
            ident,
            name: DeclarationName::Exact,
            body: DeclarationBody::Unit,
        })
    }
}

/// Parses variants, allowing commas to be omitted between delimiter-bounded forms.
fn parse_enum_variants(input: ParseStream) -> syn::Result<Vec<ParsedEnumVariant>> {
    let mut variants = Vec::new();

    while !input.is_empty() {
        let mut attrs = input.call(Attribute::parse_outer)?;
        let options = take_options(&mut attrs)?;
        let visibility = parse_visibility(input)?;
        let keyword = parse_declaration_keyword(input)?;
        let mut kind = if input.peek(Token![|]) {
            EnumVariant::Implicit(Box::new(input.parse()?))
        } else if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            let mut fields = parse_type_list(&content)?;
            if fields.len() != 1 {
                return Err(content.error("an implicit variant accepts exactly one type"));
            }
            if input.peek(Brace) {
                return Err(
                    input.error("use `|Type| { ... }` to define an explicitly named payload type")
                );
            }
            EnumVariant::Implicit(Box::new(fields.pop().expect("checked one field above")))
        } else {
            let ident = Ident::parse_any(input)?;

            if input.peek(Token![|]) {
                EnumVariant::Tuple {
                    ident,
                    fields: vec![input.parse()?],
                }
            } else if input.peek(Brace) {
                let content;
                syn::braced!(content in input);
                EnumVariant::Generated {
                    ident: ident.clone(),
                    declaration: Box::new(parse_declaration(
                        Vec::new(),
                        OptionOverrides::default(),
                        visibility.clone(),
                        keyword,
                        ident,
                        DeclarationName::Relative,
                        &content,
                    )?),
                }
            } else if input.peek(Paren) {
                let content;
                parenthesized!(content in input);
                let fields = parse_type_list(&content)?;
                if input.peek(Brace) {
                    return Err(input.error(
                        "use `Name |Type| { ... }` to name an explicitly generated payload type",
                    ));
                }
                EnumVariant::Tuple { ident, fields }
            } else {
                EnumVariant::Unit(ident)
            }
        };

        if let Some(keyword) = keyword
            && !matches!(&kind, EnumVariant::Generated { .. })
        {
            let span = match keyword {
                DeclarationKeyword::Struct(span) | DeclarationKeyword::Enum(span) => span,
            };
            return Err(Error::new(
                span,
                "`struct` and `enum` can annotate only an inline generated declaration here",
            ));
        }

        let mut nested = enum_variant_declarations_mut(&mut kind);
        if !nested.is_empty() {
            let mut variant_attrs = Vec::with_capacity(attrs.len());
            for attribute in attrs {
                if is_derive(&attribute) || is_underive(&attribute) {
                    for declaration in &mut nested {
                        declaration.attrs.push(attribute.clone());
                    }
                } else {
                    variant_attrs.push(attribute);
                }
            }
            attrs = variant_attrs;
        } else if let Some(modifier) = attrs
            .iter()
            .find(|attribute| is_derive(attribute) || is_underive(attribute))
        {
            return Err(Error::new_spanned(
                modifier,
                "a local derive modifier requires an inline generated payload type",
            ));
        }

        variants.push(ParsedEnumVariant {
            attrs,
            options,
            visibility,
            kind,
        });
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
    }

    Ok(variants)
}

/// Returns declarations generated directly by one enum variant payload.
fn enum_variant_declarations_mut(kind: &mut EnumVariant) -> Vec<&mut Declaration> {
    match kind {
        EnumVariant::Implicit(ty) => nested_declarations_mut(ty),
        EnumVariant::Tuple { fields, .. } => fields
            .iter_mut()
            .flat_map(nested_declarations_mut)
            .collect(),
        EnumVariant::Generated { declaration, .. } => vec![declaration],
        EnumVariant::Unit(_) => Vec::new(),
    }
}

/// Lowers a declaration after recursively lowering all nested declarations.
fn lower_declaration(
    declaration: Declaration,
    options: Options,
    inclusion_owner_active: bool,
) -> syn::Result<LoweredDeclaration> {
    let Declaration {
        attrs,
        options: local_options,
        visibility,
        ident,
        name: _,
        body,
    } = declaration;
    let options = options.with(local_options);
    let visibility = effective_visibility(visibility, options.public);

    match body {
        DeclarationBody::Unit => lower_unit(attrs, visibility, ident),
        DeclarationBody::Struct(fields) => lower_struct(attrs, visibility, ident, fields, options),
        DeclarationBody::Tuple(fields) => lower_tuple(attrs, visibility, ident, fields, options),
        DeclarationBody::Enum(variants) => lower_enum(
            attrs,
            visibility,
            ident,
            variants,
            options,
            inclusion_owner_active,
        ),
    }
}

/// Resolves one explicit visibility against the current default-public option.
fn effective_visibility(visibility: Option<Visibility>, public: bool) -> Visibility {
    visibility.unwrap_or_else(|| {
        if public {
            parse_quote!(pub)
        } else {
            Visibility::Inherited
        }
    })
}

/// Emits one public nominal unit type.
fn lower_unit(
    attrs: Vec<Attribute>,
    visibility: Visibility,
    ident: Ident,
) -> syn::Result<LoweredDeclaration> {
    let macro_attrs = conditional_compilation_attrs(&attrs);
    let documentation = LitStr::new(
        &format!("Unit struct generated by `strutuct!` for `{ident}`."),
        ident.span(),
    );
    let items = vec![
        quote! {
            #[allow(dead_code)]
            #(#attrs)*
            #[doc = #documentation]
            #visibility struct #ident;
        },
        emit_unit_macro(&ident, &macro_attrs),
    ];

    Ok(LoweredDeclaration {
        items,
        ident: ident.clone(),
        algebra: AlgebraicType {
            ty: ident.to_token_stream(),
            shape: AlgebraicShape::Unit,
            conditional_attrs: macro_attrs,
            value_constructor: Some(ident),
            declaration_kind: Some(DeclarationKind::Unit),
        },
    })
}

/// Emits a public struct after collecting declarations from every field type.
fn lower_struct(
    attrs: Vec<Attribute>,
    visibility: Visibility,
    ident: Ident,
    fields: Vec<StructField>,
    options: Options,
) -> syn::Result<LoweredDeclaration> {
    let macro_attrs = conditional_compilation_attrs(&attrs);
    let mut items = Vec::new();
    let mut lowered_fields = Vec::with_capacity(fields.len());
    let mut components = Vec::with_capacity(fields.len());

    for field in fields {
        let field_options = options.with(field.options);
        let lowered = lower_type(field.ty, field_options, &ident, false)?;
        items.extend(lowered.items);
        let field_attrs = field.attrs;
        let field_visibility = effective_visibility(field.visibility, field_options.public);
        let field_ident = field.ident;
        let field_ty = lowered.ty;
        components.push(ProductComponent {
            selector: ProductSelector::Named(field_ident.clone()),
            ty: lowered.algebra,
        });
        let field_documentation = LitStr::new(
            &format!("Field `{ident}::{field_ident}` generated by `strutuct!`."),
            field_ident.span(),
        );
        lowered_fields.push(quote! {
            #(#field_attrs)*
            #[doc = #field_documentation]
            #field_visibility #field_ident: #field_ty
        });
    }

    let documentation = LitStr::new(
        &format!("Struct generated by `strutuct!` for `{ident}`."),
        ident.span(),
    );
    items.push(quote! {
        #[allow(dead_code)]
        #(#attrs)*
        #[doc = #documentation]
        #visibility struct #ident {
            #(#lowered_fields),*
        }
    });
    items.push(emit_struct_macro(&ident, &macro_attrs));

    Ok(LoweredDeclaration {
        items,
        ident: ident.clone(),
        algebra: AlgebraicType {
            ty: ident.to_token_stream(),
            shape: AlgebraicShape::Product(components),
            conditional_attrs: macro_attrs,
            value_constructor: None,
            declaration_kind: Some(DeclarationKind::Struct),
        },
    })
}

/// Emits a public tuple struct after collecting declarations from every field type.
fn lower_tuple(
    attrs: Vec<Attribute>,
    visibility: Visibility,
    ident: Ident,
    fields: Vec<TypeExpression>,
    options: Options,
) -> syn::Result<LoweredDeclaration> {
    let macro_attrs = conditional_compilation_attrs(&attrs);
    let mut items = Vec::new();
    let mut lowered_fields = Vec::with_capacity(fields.len());
    let mut components = Vec::with_capacity(fields.len());
    let field_visibility = effective_visibility(None, options.public);

    for (index, field) in fields.into_iter().enumerate() {
        let lowered = lower_type(field, options, &ident, false)?;
        items.extend(lowered.items);
        let field_ty = lowered.ty;
        components.push(ProductComponent {
            selector: ProductSelector::Positional(index),
            ty: lowered.algebra,
        });
        let field_documentation = LitStr::new(
            &format!("Field `{ident}::{index}` generated by `strutuct!`."),
            ident.span(),
        );
        lowered_fields.push(quote! {
            #[doc = #field_documentation]
            #field_visibility #field_ty
        });
    }

    let documentation = LitStr::new(
        &format!("Tuple struct generated by `strutuct!` for `{ident}`."),
        ident.span(),
    );
    items.push(quote! {
        #[allow(dead_code)]
        #(#attrs)*
        #[doc = #documentation]
        #visibility struct #ident(#(#lowered_fields),*);
    });
    items.push(emit_tuple_macro(&ident, &macro_attrs));

    Ok(LoweredDeclaration {
        items,
        ident: ident.clone(),
        algebra: AlgebraicType {
            ty: ident.to_token_stream(),
            shape: AlgebraicShape::Product(components),
            conditional_attrs: macro_attrs,
            value_constructor: Some(ident),
            declaration_kind: Some(DeclarationKind::Tuple),
        },
    })
}

/// Emits a public enum and a recursively delegating constructor macro.
fn lower_enum(
    attrs: Vec<Attribute>,
    visibility: Visibility,
    ident: Ident,
    variants: Vec<ParsedEnumVariant>,
    options: Options,
    inclusion_owner_active: bool,
) -> syn::Result<LoweredDeclaration> {
    let macro_attrs = conditional_compilation_attrs(&attrs);
    let mut items = Vec::new();
    let mut lowered_variants = Vec::with_capacity(variants.len());
    let mut macro_arms = Vec::new();
    let mut algebraic_variants = Vec::new();

    for variant in variants {
        let ParsedEnumVariant {
            attrs: variant_attrs,
            options: variant_overrides,
            visibility: variant_visibility,
            kind,
        } = variant;
        let variant_options = options.with(variant_overrides);
        let product_variants = variant_options.product_variants;
        match kind {
            EnumVariant::Implicit(ty) => {
                let mut ty = *ty;
                apply_visibility_to_nested_type(&mut ty, variant_visibility);
                let lowered = lower_type(ty, variant_options, &ident, variant_options.inclusions)?;
                items.extend(lowered.items);
                let payload_name = lowered.name.clone().ok_or_else(|| {
                    Error::new(
                        ident.span(),
                        "cannot derive an implicit variant name from this type; name the variant explicitly",
                    )
                })?;
                let variant_ident =
                    concatenate(&[&payload_name, &ident], variant_options.reverse_concat);
                let payload_ty = lowered.ty.clone();
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
                    match lowered.algebra.declaration_kind {
                        Some(DeclarationKind::Unit) => {
                            macro_arms.push(quote! {
                                (#payload_name) => {
                                    #ident::#variant_ident(#payload_name)
                                };
                            });
                        }
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

                algebraic_variants.push(CoproductVariant {
                    selector: payload_name,
                    variant: variant_ident,
                    payload: lowered.algebra,
                    constructor: CoproductConstructor::Unary,
                    conditional_attrs: conditional_compilation_attrs(&variant_attrs),
                    inclusions: variant_options.inclusions,
                    reverse_concat: variant_options.reverse_concat,
                });
            }
            EnumVariant::Tuple {
                ident: variant_ident,
                mut fields,
            } => {
                for field in &mut fields {
                    apply_visibility_to_nested_type(field, variant_visibility.clone());
                }
                let composes_nested_enum = variant_options.inclusions && fields.len() == 1;
                let mut fields = fields
                    .into_iter()
                    .map(|field| lower_type(field, variant_options, &ident, composes_nested_enum))
                    .collect::<syn::Result<Vec<_>>>()?;
                for field in &mut fields {
                    items.append(&mut field.items);
                }
                if let [field] = fields.as_slice()
                    && !field.wrapped
                    && let (Some(payload_name), Some(kind)) =
                        (&field.name, field.algebra.declaration_kind)
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
                let lowered_fields = fields
                    .iter()
                    .map(|field| field.ty.clone())
                    .collect::<Vec<_>>();
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

                let constructor = if product || fields.len() == 1 {
                    CoproductConstructor::Unary
                } else {
                    CoproductConstructor::Tuple(fields.len())
                };
                let payload = match fields.len() {
                    0 => unit_algebra(),
                    1 => {
                        fields
                            .into_iter()
                            .next()
                            .expect("checked one field above")
                            .algebra
                    }
                    _ => {
                        let components = fields
                            .into_iter()
                            .enumerate()
                            .map(|(index, field)| ProductComponent {
                                selector: ProductSelector::Positional(index),
                                ty: field.algebra,
                            })
                            .collect();
                        anonymous_product(lowered_fields, components)
                    }
                };
                algebraic_variants.push(CoproductVariant {
                    selector: variant_ident.clone(),
                    variant: variant_ident,
                    payload,
                    constructor,
                    conditional_attrs: conditional_compilation_attrs(&variant_attrs),
                    inclusions: variant_options.inclusions,
                    reverse_concat: variant_options.reverse_concat,
                });
            }
            EnumVariant::Generated {
                ident: variant_ident,
                declaration,
            } => {
                let Declaration {
                    attrs: declaration_attrs,
                    options: declaration_options,
                    visibility: declaration_visibility,
                    ident: declaration_ident,
                    name: declaration_name,
                    body,
                } = *declaration;

                match (product_variants, body) {
                    (false, DeclarationBody::Struct(fields)) => {
                        // This declaration becomes a struct-like variant rather than an item.
                        // Its attributes have already propagated into generated field types;
                        // retargeting them onto the variant would change their syntactic target.
                        let mut lowered_fields = Vec::with_capacity(fields.len());
                        let mut components = Vec::with_capacity(fields.len());
                        let mut field_names = Vec::with_capacity(fields.len());
                        for field in fields {
                            let field_options = variant_options.with(field.options);
                            let lowered = lower_type(field.ty, field_options, &ident, false)?;
                            items.extend(lowered.items);
                            let field_attrs = field.attrs;
                            let field_ident = field.ident;
                            let field_ty = lowered.ty;
                            components.push(ProductComponent {
                                selector: ProductSelector::Named(field_ident.clone()),
                                ty: lowered.algebra,
                            });
                            field_names.push(field_ident.clone());
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
                            #[doc = #variant_documentation]
                            #variant_ident { #(#lowered_fields),* }
                        });
                        let payload_types = components
                            .iter()
                            .map(|component| component.ty.ty.clone())
                            .collect();
                        algebraic_variants.push(CoproductVariant {
                            selector: variant_ident.clone(),
                            variant: variant_ident,
                            payload: anonymous_product(payload_types, components),
                            constructor: CoproductConstructor::Struct(field_names),
                            conditional_attrs: conditional_compilation_attrs(&variant_attrs),
                            inclusions: variant_options.inclusions,
                            reverse_concat: variant_options.reverse_concat,
                        });
                    }
                    (_, body) => {
                        let lowered = lower_nested_declaration(
                            Declaration {
                                attrs: declaration_attrs,
                                options: declaration_options,
                                visibility: declaration_visibility,
                                ident: declaration_ident,
                                name: declaration_name,
                                body,
                            },
                            &ident,
                            variant_options,
                            variant_options.inclusions,
                        )?;
                        items.extend(lowered.items);
                        let payload_ident = lowered.ident.clone();
                        let kind = lowered
                            .algebra
                            .declaration_kind
                            .expect("lowered declarations retain their constructor kind");
                        macro_arms.push(nested_variant_arm(
                            &ident,
                            &variant_ident,
                            &variant_ident,
                            &payload_ident,
                            kind,
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
                        algebraic_variants.push(CoproductVariant {
                            selector: variant_ident.clone(),
                            variant: variant_ident,
                            payload: lowered.algebra,
                            constructor: CoproductConstructor::Unary,
                            conditional_attrs: conditional_compilation_attrs(&variant_attrs),
                            inclusions: variant_options.inclusions,
                            reverse_concat: variant_options.reverse_concat,
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
                algebraic_variants.push(CoproductVariant {
                    selector: variant_ident.clone(),
                    variant: variant_ident,
                    payload: unit_algebra(),
                    constructor: CoproductConstructor::Unit,
                    conditional_attrs: conditional_compilation_attrs(&variant_attrs),
                    inclusions: variant_options.inclusions,
                    reverse_concat: variant_options.reverse_concat,
                });
            }
        }
    }

    let documentation = LitStr::new(
        &format!("Enum generated by `strutuct!` for `{ident}`."),
        ident.span(),
    );
    let algebra = AlgebraicType {
        ty: ident.to_token_stream(),
        shape: AlgebraicShape::Coproduct(algebraic_variants),
        conditional_attrs: macro_attrs.clone(),
        value_constructor: None,
        declaration_kind: Some(DeclarationKind::Enum),
    };
    items.push(quote! {
        #[allow(dead_code)]
        #(#attrs)*
        #[doc = #documentation]
        #visibility enum #ident {
            #(#lowered_variants),*
        }
    });
    items.push(emit_enum_macro(&ident, &macro_arms, &macro_attrs));
    if !inclusion_owner_active {
        let flattened = algebra.flattened_coproduct();
        debug_assert!(flattened.len() <= algebra.node_count());
        items.extend(emit_inclusion_functions(&ident, &visibility, flattened)?);
    }

    Ok(LoweredDeclaration {
        items,
        ident,
        algebra,
    })
}

/// Builds the anonymous terminal product used as a unit-variant payload.
fn unit_algebra() -> AlgebraicType {
    AlgebraicType {
        ty: quote!(()),
        shape: AlgebraicShape::Unit,
        conditional_attrs: Vec::new(),
        value_constructor: None,
        declaration_kind: None,
    }
}

/// Builds an anonymous tuple product from already lowered components.
fn anonymous_product(types: Vec<TokenStream>, components: Vec<ProductComponent>) -> AlgebraicType {
    AlgebraicType {
        ty: quote!((#(#types,)*)),
        shape: AlgebraicShape::Product(components),
        conditional_attrs: Vec::new(),
        value_constructor: None,
        declaration_kind: None,
    }
}

/// Returns one stable hygienic binding used inside generated inclusion functions.
fn inclusion_value_ident(_span: Span) -> Ident {
    Ident::new("value", Span::mixed_site())
}

/// Merges conditional attributes without applying the same condition twice.
fn combined_conditional_attrs(left: &[Attribute], right: &[Attribute]) -> Vec<Attribute> {
    let mut combined = left.to_vec();
    for attribute in right {
        let spelling = attribute.to_token_stream().to_string();
        if !combined
            .iter()
            .any(|existing| existing.to_token_stream().to_string() == spelling)
        {
            combined.push(attribute.clone());
        }
    }
    combined
}

/// Concatenates one root name with every selector on a flattened coproduct path.
fn inclusion_name(root: &Ident, steps: &[InclusionStep]) -> Ident {
    steps.iter().fold(root.clone(), |prefix, step| {
        concatenate(&[&prefix, &step.selector], step.reverse_concat)
    })
}

/// Emits consuming free functions for a flattened family of coproduct injections.
fn emit_inclusion_functions(
    root: &Ident,
    visibility: &Visibility,
    inclusions: Vec<InclusionPath>,
) -> syn::Result<Vec<TokenStream>> {
    inclusions
        .into_iter()
        .map(|inclusion| {
            let ident = inclusion_name(root, &inclusion.steps);
            if inclusion
                .value_constructor
                .as_ref()
                .is_some_and(|constructor| constructor.unraw() == ident.unraw())
            {
                return Err(Error::new(
                    ident.span(),
                    format!(
                        "generated inclusion `{ident}` collides with the value constructor of its tuple or unit payload; give the payload a distinct `|Type|` name or disable this inclusion branch"
                    ),
                ));
            }

            let attrs = inclusion.conditional_attrs;
            let value = inclusion.value;
            let documentation = LitStr::new(
                &format!(
                    "Injects the `{}` constructor occurrence into `{root}`.",
                    inclusion
                        .steps
                        .iter()
                        .map(|step| step.selector.unraw().to_string())
                        .collect::<Vec<_>>()
                        .join("::")
                ),
                ident.span(),
            );
            let function = if let Some(parameter) = inclusion.parameter {
                let argument = inclusion_value_ident(ident.span());
                quote! {
                    #(#attrs)*
                    #[doc = #documentation]
                    #[allow(dead_code, non_snake_case)]
                    #visibility fn #ident(#argument: #parameter) -> #root {
                        #value
                    }
                }
            } else {
                quote! {
                    #(#attrs)*
                    #[doc = #documentation]
                    #[allow(dead_code, non_snake_case)]
                    #visibility fn #ident() -> #root {
                        #value
                    }
                }
            };
            Ok(function)
        })
        .collect()
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
        DeclarationKind::Unit => quote! {
            (#selector) => {
                #parent::#variant(#payload)
            };
        },
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

/// Emits a same-name macro that constructs a generated unit struct.
fn emit_unit_macro(ident: &Ident, attrs: &[Attribute]) -> TokenStream {
    let documentation = LitStr::new(
        &format!("Constructs a `{ident}` value generated by `strutuct!`."),
        ident.span(),
    );
    quote! {
        #(#attrs)*
        #[doc = #documentation]
        #[allow(unused_macros)]
        macro_rules! #ident {
            () => {
                #ident
            };
        }
    }
}

/// Lowers a type expression and marks wrapped recursive edges as terminal.
fn lower_type(
    expression: TypeExpression,
    options: Options,
    parent: &Ident,
    inclusion_owner_active: bool,
) -> syn::Result<LoweredType> {
    let TypeExpression { base, wrappers } = expression;
    let wrapped = !wrappers.is_empty();
    let mut lowered = match base {
        TypeBase::Rust { ty, nested } => {
            let mut ty = *ty;
            let mut items = Vec::new();
            let mut embedded = Vec::new();
            for nested in nested {
                let declaration =
                    lower_nested_declaration(nested.declaration, parent, options, false)?;
                ReplacePlaceholder {
                    placeholder: &nested.placeholder,
                    replacement: &declaration.ident,
                }
                .visit_type_mut(&mut ty);
                items.extend(declaration.items);
                embedded.push(declaration.algebra);
            }
            let name = type_name(&ty);
            let ty = ty.into_token_stream();
            LoweredType {
                name,
                ty: ty.clone(),
                items,
                algebra: AlgebraicType {
                    ty,
                    shape: AlgebraicShape::Rust(embedded),
                    conditional_attrs: Vec::new(),
                    value_constructor: None,
                    declaration_kind: None,
                },
                wrapped: false,
            }
        }
        TypeBase::Nested(declaration) => {
            let declaration = lower_nested_declaration(
                *declaration,
                parent,
                options,
                inclusion_owner_active && !wrapped,
            )?;
            LoweredType {
                ty: declaration.ident.to_token_stream(),
                name: Some(declaration.ident.clone()),
                items: declaration.items,
                algebra: declaration.algebra,
                wrapped: false,
            }
        }
    };

    if wrapped {
        lowered.wrapped = true;
    }
    for wrapper in wrappers {
        let inner_ty = lowered.ty;
        let ty = match wrapper {
            TypeWrapper::Option => quote!(::core::option::Option<#inner_ty>),
            TypeWrapper::Box => quote!(::std::boxed::Box<#inner_ty>),
        };
        let inner = lowered.algebra;
        let conditional_attrs = inner.conditional_attrs.clone();
        lowered.ty = ty.clone();
        lowered.algebra = AlgebraicType {
            ty,
            shape: AlgebraicShape::Wrapped {
                wrapper,
                inner: Box::new(inner),
            },
            conditional_attrs,
            value_constructor: None,
            declaration_kind: None,
        };
    }

    Ok(lowered)
}

/// Resolves a nested declaration name against its generated parent before lowering.
fn lower_nested_declaration(
    mut declaration: Declaration,
    parent: &Ident,
    options: Options,
    inclusion_owner_active: bool,
) -> syn::Result<LoweredDeclaration> {
    if matches!(declaration.name, DeclarationName::Relative) {
        let options = options.with(declaration.options);
        declaration.ident = concatenate(&[parent, &declaration.ident], options.reverse_concat);
        declaration.name = DeclarationName::Exact;
    }
    lower_declaration(declaration, options, inclusion_owner_active)
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
fn emit_struct_macro(ident: &Ident, attrs: &[Attribute]) -> TokenStream {
    let documentation = LitStr::new(
        &format!("Constructs a `{ident}` value generated by `strutuct!`."),
        ident.span(),
    );
    quote! {
        #(#attrs)*
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
fn emit_tuple_macro(ident: &Ident, attrs: &[Attribute]) -> TokenStream {
    let documentation = LitStr::new(
        &format!("Constructs a `{ident}` value generated by `strutuct!`."),
        ident.span(),
    );
    quote! {
        #(#attrs)*
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
fn emit_enum_macro(
    ident: &Ident,
    specialized_arms: &[TokenStream],
    attrs: &[Attribute],
) -> TokenStream {
    let documentation = LitStr::new(
        &format!("Constructs an `{ident}` value generated by `strutuct!`."),
        ident.span(),
    );
    quote! {
        #(#attrs)*
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

/// Concatenates identifiers in the selected order while retaining a source span.
fn concatenate(parts: &[&Ident], reverse: bool) -> Ident {
    let ordered: Vec<_> = if reverse {
        parts.iter().rev().copied().collect()
    } else {
        parts.to_vec()
    };
    let spelling = ordered
        .iter()
        .map(|part| part.unraw().to_string())
        .collect::<String>();
    let span = ordered
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

    /// Returns the attributes emitted on one generated nominal declaration.
    fn declaration_attrs<'a>(file: &'a syn::File, name: &str) -> &'a [Attribute] {
        file.items
            .iter()
            .find_map(|item| match item {
                syn::Item::Enum(item) if item.ident == name => Some(item.attrs.as_slice()),
                syn::Item::Struct(item) if item.ident == name => Some(item.attrs.as_slice()),
                _ => None,
            })
            .expect("generated declaration")
    }

    /// Returns normalized trait paths from a generated declaration's derive list.
    fn derived_traits(file: &syn::File, name: &str) -> Vec<String> {
        declaration_attrs(file, name)
            .iter()
            .filter(|attribute| is_derive(attribute))
            .flat_map(|attribute| parse_derive_paths(attribute).expect("derive paths"))
            .map(|path| path.to_token_stream().to_string())
            .collect()
    }

    /// Returns literal documentation strings attached to an item or field.
    fn documentation(attrs: &[Attribute]) -> Vec<String> {
        attrs
            .iter()
            .filter_map(|attribute| {
                let syn::Meta::NameValue(meta) = &attribute.meta else {
                    return None;
                };
                if !meta.path.is_ident("doc") {
                    return None;
                }
                let syn::Expr::Lit(expression) = &meta.value else {
                    return None;
                };
                let syn::Lit::Str(text) = &expression.lit else {
                    return None;
                };
                Some(text.value())
            })
            .collect()
    }

    /// Compares generated and expected items without depending on whitespace.
    fn assert_expands_to(input: &str, expected: &str) {
        assert_eq!(
            expand(input).into_token_stream().to_string(),
            rust(expected).into_token_stream().to_string()
        );
    }

    /// Hoists a relatively named nested enum before the struct that refers to it.
    #[test]
    fn lowers_nested_enum_in_struct_field() {
        assert_expands_to(
            "S a: A { A1, A2, A3 }, b: B,",
            r#"
                #[allow(dead_code)]
                #[doc = "Enum generated by `strutuct!` for `SA`."]
                pub enum SA {
                    #[doc = "Variant `SA::A1` generated by `strutuct!`."]
                    A1,
                    #[doc = "Variant `SA::A2` generated by `strutuct!`."]
                    A2,
                    #[doc = "Variant `SA::A3` generated by `strutuct!`."]
                    A3
                }
                #[doc = "Constructs an `SA` value generated by `strutuct!`."]
                #[allow(unused_macros)]
                macro_rules! SA {
                    ($($variant:tt)+) => { SA:: $($variant)+ };
                }
                #[allow(dead_code)]
                #[doc = "Struct generated by `strutuct!` for `S`."]
                pub struct S {
                    #[doc = "Field `S::a` generated by `strutuct!`."]
                    pub a: SA,
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

    /// Extends family derives for one nested declaration and its descendants.
    #[test]
    fn local_derive_extends_the_nested_branch() {
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
        assert_eq!(derives_for("RootBranch"), ["Clone", "Debug", "Default"]);
        assert_eq!(derives_for("RootBranchLeaf"), ["Clone", "Debug", "Default"]);
    }

    /// Hoists an inline declaration from nested generic arguments before its consumer.
    #[test]
    fn lowers_declarations_inside_generic_types() {
        let expanded = expand(
            "#[derive(Debug)] Root value: Delimited<Option<Content { Empty, Value(String) }>>,",
        )
        .into_token_stream()
        .to_string();

        assert!(expanded.contains("pub enum RootContent"));
        assert!(expanded.contains("derive (Debug)"));
        assert!(expanded.contains("pub value : Delimited < Option < RootContent > >"));
        assert!(expanded.find("pub enum RootContent") < expanded.find("pub struct Root"));
    }

    /// Accepts Rust-like root braces, visibility, and shape keywords.
    #[test]
    fn accepts_braced_keyword_declarations_and_visibility() {
        let expanded = expand("pub(crate) struct Product { priv hidden: u8, pub shown: u8, }")
            .into_token_stream()
            .to_string();

        assert!(expanded.contains("pub (crate) struct Product"));
        assert!(expanded.contains("hidden : u8"));
        assert!(!expanded.contains("pub hidden : u8"));
        assert!(expanded.contains("pub shown : u8"));

        let private = expand("#[strutuct(public = false)] struct Private { field: u8 }")
            .into_token_stream()
            .to_string();
        assert!(private.contains("struct Private"));
        assert!(!private.contains("pub struct Private"));
        assert!(!private.contains("pub field : u8"));
    }

    /// Diagnoses decorative keywords that disagree with the inferred body shape.
    #[test]
    fn validates_decorative_declaration_keywords() {
        for (input, expected) in [
            (
                "struct Wrong { A, B }",
                "`struct` does not match the inferred enum declaration",
            ),
            (
                "enum Wrong { value: u8 }",
                "`enum` does not match the inferred struct declaration",
            ),
            (
                "struct Root { values: Vec<struct Wrong { A, B }> }",
                "`struct` does not match the inferred enum declaration",
            ),
        ] {
            let error = parse2::<Invocation>(input.parse::<TokenStream>().expect("tokens"))
                .err()
                .expect("the mismatched keyword must be rejected");

            assert_eq!(error.to_string(), expected);
        }
    }

    /// Applies public and concatenation options to one nested branch only.
    #[test]
    fn applies_configuration_globally_and_locally() {
        let expanded = expand(
            "#[strutuct(public = false)] enum Root { #[strutuct(public = true, reverse_concat = true)] pub Branch { Leaf { A, B } }, Regular { A, B } }",
        )
        .into_token_stream()
        .to_string();

        assert!(expanded.contains("enum Root"));
        assert!(!expanded.contains("pub enum Root"));
        assert!(expanded.contains("pub enum BranchRoot"));
        assert!(expanded.contains("pub enum LeafBranchRoot"));
        assert!(expanded.contains("enum RootRegular"));
        assert!(!expanded.contains("pub enum RootRegular"));
    }

    /// Gives declaration-local options precedence over the forwarded invocation defaults.
    #[test]
    fn applies_forwarded_attributes_as_invocation_defaults() {
        let private = expand("#[strutuct(public = false)] ; struct Private { value: u8 }")
            .into_token_stream()
            .to_string();
        assert!(!private.contains("pub struct Private"));
        assert!(!private.contains("pub value : u8"));

        let overridden = expand(
            "#[strutuct(public = false)] ; #[strutuct(public = true)] struct Public { value: u8 }",
        )
        .into_token_stream()
        .to_string();
        assert!(overridden.contains("pub struct Public"));
        assert!(overridden.contains("pub value : u8"));
    }

    /// Parses attributes, keywords, and local configuration inside a generic argument.
    #[test]
    fn configures_declarations_inside_generic_types() {
        let expanded = expand(
            "struct Holder { values: Vec<#[strutuct(public = false, reverse_concat = true)] enum Choice { Nested { A, B } }>, }",
        )
        .into_token_stream()
        .to_string();

        assert!(expanded.contains("pub values : Vec < ChoiceHolder >"));
        assert!(expanded.contains("enum ChoiceHolder"));
        assert!(!expanded.contains("pub enum ChoiceHolder"));
        assert!(expanded.contains("enum NestedChoiceHolder"));
        assert!(!expanded.contains("pub enum NestedChoiceHolder"));
    }

    /// Duplicates arbitrary root declaration attributes onto generated descendants.
    #[test]
    fn propagates_all_outer_declaration_attributes() {
        let expanded = expand("#[repr(C)] struct Root { child: enum Child { A, B }, }")
            .into_token_stream()
            .to_string();

        assert_eq!(expanded.matches("# [repr (C)]").count(), 2);
    }

    /// Keeps documentation on its source declaration or field instead of inheriting it.
    #[test]
    fn keeps_documentation_local() {
        let expanded = expand(
            r#"
                #[doc = "root only"]
                Root
                #[doc = "field only"]
                child: #[doc = "child only"] Child { A, B },
            "#,
        );

        let root_docs = documentation(declaration_attrs(&expanded, "Root"));
        let child_docs = documentation(declaration_attrs(&expanded, "RootChild"));
        let field_docs = expanded
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Struct(item) if item.ident == "Root" => item
                    .fields
                    .iter()
                    .find(|field| field.ident.as_ref().is_some_and(|ident| ident == "child"))
                    .map(|field| documentation(&field.attrs)),
                _ => None,
            })
            .expect("generated field");

        assert!(root_docs.iter().any(|doc| doc == "root only"));
        assert!(!child_docs.iter().any(|doc| doc == "root only"));
        assert!(child_docs.iter().any(|doc| doc == "child only"));
        assert!(field_docs.iter().any(|doc| doc == "field only"));
    }

    /// Subtracts local traits and propagates the reduced derive list down the branch.
    #[test]
    fn underive_reduces_the_inherited_branch() {
        let expanded = expand(
            "#[derive(Debug, Clone, Eq)] #[underive(Eq)] Root #[underive(Clone, Hash)] child: Child { leaf: Leaf { value: u8 } },",
        );

        assert_eq!(derived_traits(&expanded, "Root"), ["Debug", "Clone"]);
        assert_eq!(derived_traits(&expanded, "RootChild"), ["Debug"]);
        assert_eq!(derived_traits(&expanded, "RootChildLeaf"), ["Debug"]);

        let enum_branch =
            expand("#[derive(Debug, Clone)] Root #[underive(Clone)] Branch { Leaf { A, B } }");
        assert_eq!(derived_traits(&enum_branch, "RootBranch"), ["Debug"]);
        assert_eq!(derived_traits(&enum_branch, "RootBranchLeaf"), ["Debug"]);

        let named_payload = expand(
            "#[derive(Debug, Clone)] Root #[underive(Clone)] Branch |Payload| { Leaf { A, B } }",
        );
        assert_eq!(derived_traits(&named_payload, "Payload"), ["Debug"]);
        assert_eq!(derived_traits(&named_payload, "PayloadLeaf"), ["Debug"]);
    }

    /// Keeps generic commas inside the type while hoisting declarations from grouped arguments.
    #[test]
    fn lowers_declarations_from_multiple_generic_arguments() {
        let expanded = expand("Root value: Either<Left { A }, (Right { B }, Option<Deep { C }>)>,")
            .into_token_stream()
            .to_string();

        for declaration in ["RootLeft", "RootRight", "RootDeep"] {
            assert!(expanded.contains(&format!("pub enum {declaration}")));
        }
        assert!(
            expanded
                .contains("pub value : Either < RootLeft , (RootRight , Option < RootDeep >) >")
        );
    }

    /// Resolves relative names through ordinary type syntax and resets at exact names.
    #[test]
    fn resolves_relative_names_across_type_contexts() {
        let expanded = expand(
            r#"
                Root
                direct: Direct { A, B },
                optional: Optional { A, B }?,
                boxed: Boxed { A, B }*,
                generic: Vec<Generic { A, B }>,
                grouped: (Grouped { A, B },),
                array: [Element { A, B }; 2],
                exact: |Exact| { child: Child { A, B } },
                existing: Existing,
            "#,
        )
        .into_token_stream()
        .to_string();

        for declaration in [
            "RootDirect",
            "RootOptional",
            "RootBoxed",
            "RootGeneric",
            "RootGrouped",
            "RootElement",
            "Exact",
            "ExactChild",
        ] {
            assert!(
                expanded.contains(&format!("pub enum {declaration}"))
                    || expanded.contains(&format!("pub struct {declaration}")),
                "missing declaration `{declaration}` in {expanded}",
            );
        }
        assert!(expanded.contains("pub direct : RootDirect"));
        assert!(expanded.contains("Option < RootOptional >"));
        assert!(expanded.contains("Box < RootBoxed >"));
        assert!(expanded.contains("Vec < RootGeneric >"));
        assert!(expanded.contains("(RootGrouped ,)"));
        assert!(expanded.contains("[RootElement ; 2]"));
        assert!(expanded.contains("pub exact : Exact"));
        assert!(expanded.contains("pub existing : Existing"));
        assert!(!expanded.contains("enum RootExisting"));
    }

    /// Does not retarget declaration attributes when a product is inlined as a variant.
    #[test]
    fn does_not_attach_declaration_attrs_to_inlined_struct_variants() {
        let expanded = expand(
            "#[derive(Debug, PartialEq)] #[repr(C)] #[strutuct(product_variants = false)] Root Branch { child: Child { A, B } }",
        );
        let root = expanded
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Enum(item) if item.ident == "Root" => Some(item),
                _ => None,
            })
            .expect("generated root enum");
        let branch = root
            .variants
            .iter()
            .find(|variant| variant.ident == "Branch")
            .expect("inlined struct variant");

        assert!(branch.attrs.iter().all(|attribute| !is_derive(attribute)));
        assert!(
            branch
                .attrs
                .iter()
                .all(|attribute| !attribute.path().is_ident("repr"))
        );
        assert_eq!(
            derived_traits(&expanded, "RootChild"),
            ["Debug", "PartialEq"]
        );
        assert!(
            declaration_attrs(&expanded, "RootChild")
                .iter()
                .any(|attribute| attribute.path().is_ident("repr"))
        );
    }

    /// Applies conditional compilation equally to declarations and constructor macros.
    #[test]
    fn guards_constructor_macros_with_declaration_cfg() {
        let expanded = expand("#[cfg(any())] Root Branch { A, B }");
        let macros = expanded
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Macro(item) => Some(item),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(macros.len(), 2);
        assert!(macros.iter().all(|item| {
            item.attrs
                .iter()
                .any(|attribute| attribute.path().is_ident("cfg"))
        }));
    }

    /// Leaves nested-looking tokens inside a type macro for that macro to interpret.
    #[test]
    fn leaves_type_macro_inputs_opaque() {
        let expanded = expand("Root value: Wrapper<type_macro!(Content { Empty })>,")
            .into_token_stream()
            .to_string();

        assert!(!expanded.contains("pub enum Content"));
        assert!(expanded.contains("Wrapper < type_macro ! (Content { Empty }) >"));
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

    /// Emits one root inclusion per constructor occurrence without enumerating factorizations.
    #[test]
    fn flattens_enum_inclusions_once_per_path() {
        let expanded = expand("#[strutuct(inclusions = true)] A B { C { D { E } } }");
        let functions = expanded
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Fn(function) => Some(function),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions
                .iter()
                .map(|function| function.sig.ident.to_string())
                .collect::<Vec<_>>(),
            ["AB", "ABC", "ABCD", "ABCDE"]
        );
        let deepest = functions
            .last()
            .expect("the leaf inclusion must be generated")
            .block
            .to_token_stream()
            .to_string();
        assert!(deepest.contains("A :: B (AB :: C (ABC :: D (ABCD :: E)))"));
    }

    /// Lets one branch enable or disable inclusion generation without affecting siblings.
    #[test]
    fn configures_inclusions_per_enum_branch() {
        let expanded = expand(
            "Root #[strutuct(inclusions = true)] Enabled { Leaf } #[strutuct(inclusions = false)] Disabled { Leaf }",
        );
        let functions = expanded
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Fn(function) => Some(function.sig.ident.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(functions, ["RootEnabled", "RootEnabledLeaf"]);
    }

    /// Applies the active concatenation order to each inclusion-path edge.
    #[test]
    fn reverses_inclusion_path_names_incrementally() {
        let expanded =
            expand("#[strutuct(inclusions = true, reverse_concat = true)] Root Branch { Leaf }");
        let functions = expanded
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Fn(function) => Some(function.sig.ident.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(functions, ["BranchRoot", "LeafBranchRoot"]);
    }

    /// Carries variant cfg conditions onto every inclusion using that variant.
    #[test]
    fn guards_inclusions_with_constructor_conditions() {
        let expanded = expand(
            "#[strutuct(inclusions = true)] Root #[cfg(any())] Hidden(String) Visible(String)",
        );
        let functions = expanded
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Fn(function) => Some(function),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(functions.len(), 2);
        assert!(
            functions[0]
                .attrs
                .iter()
                .any(|attribute| attribute.path().is_ident("cfg"))
        );
        assert!(
            !functions[1]
                .attrs
                .iter()
                .any(|attribute| attribute.path().is_ident("cfg"))
        );
    }

    /// Stops root composition across products while retaining the nested enum's own maps.
    #[test]
    fn stops_inclusion_join_at_product_boundaries() {
        let expanded =
            expand("#[strutuct(inclusions = true)] Root Branch { child: Child { Leaf } }");
        let functions = expanded
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Fn(function) => Some(function.sig.ident.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(functions, ["RootBranchChildLeaf", "RootBranch"]);
        let leaf = expanded
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Fn(function) if function.sig.ident == "RootBranchChildLeaf" => {
                    Some(function)
                }
                _ => None,
            })
            .expect("the nested enum keeps its local leaf inclusion");
        assert_eq!(
            leaf.sig.output.to_token_stream().to_string(),
            "-> RootBranchChild"
        );
    }

    /// Diagnoses the value-namespace collision caused by tuple and unit constructors.
    #[test]
    fn diagnoses_inclusion_and_payload_constructor_collisions() {
        let tokens = "#[strutuct(inclusions = true)] Root Pair { (u8, u8) }"
            .parse()
            .expect("tokens");
        let expanded = strutuct(tokens).to_string();

        assert!(expanded.contains("generated inclusion `RootPair` collides"));
    }

    /// Keeps parentheses exclusively for payloads instead of generated type names.
    #[test]
    fn rejects_the_old_parenthesized_generated_type_spelling() {
        for (input, expected) in [
            (
                "Expr Named(Payload) { A, B }",
                "use `Name |Type| { ... }` to name an explicitly generated payload type",
            ),
            (
                "Expr (Payload) { A, B }",
                "use `|Type| { ... }` to define an explicitly named payload type",
            ),
        ] {
            let error = parse2::<Invocation>(input.parse::<TokenStream>().expect("tokens"))
                .err()
                .expect("the old contextual spelling must be rejected");

            assert_eq!(error.to_string(), expected);
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
