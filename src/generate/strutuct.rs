//! Nested algebraic declarations replacing one ordinary Rust item shape.

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
    Attribute, Error, Ident, LitStr, Path, Token, Type, Visibility, braced,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    parse_quote_spanned, parse2,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Brace, Paren},
};

use syn::Meta;

/// Options explicitly set by `#[strutuct(...)]` attributes on one object.
///
/// ```rust,ignore
/// #[strutuct(pub(crate), reverse_concat)]
/// // visibility: Some(pub(crate)), reverse_concat: Some(true), everything else inherited
/// ```
#[derive(Clone, Default)]
struct OptionOverrides {
    /// Per-object override for generated enum inclusion functions.
    inclusions: Option<bool>,
    /// Per-object override for packed product variants.
    product_variants: Option<bool>,
    /// Visibility inherited by this object and every declaration and field below it.
    visibility: Option<Visibility>,
    /// Per-object override for identifier concatenation order.
    reverse_concat: Option<bool>,
    /// Whether this branch permits declarations and members without documentation.
    allow_missing_docs: Option<bool>,
}

/// Uses the same option grammar in parameter envelopes and local attributes.
impl Parse for OptionOverrides {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut options = Self::default();
        options.parse_entries(input)?;
        Ok(options)
    }
}

impl OptionOverrides {
    /// Combines inherited and local overrides, preferring the local values.
    fn with(self, local: Self) -> Self {
        Self {
            inclusions: local.inclusions.or(self.inclusions),
            product_variants: local.product_variants.or(self.product_variants),
            visibility: local.visibility.or(self.visibility),
            reverse_concat: local.reverse_concat.or(self.reverse_concat),
            allow_missing_docs: local.allow_missing_docs.or(self.allow_missing_docs),
        }
    }

    /// Reports whether the attribute is a `#[strutuct(...)]` configuration attribute.
    fn configures(attribute: &Attribute) -> bool {
        attribute.path().is_ident("strutuct")
    }

    /// Removes `#[strutuct(...)]` attributes from `attrs` and collects their options.
    fn take_from(attrs: &mut Vec<Attribute>) -> syn::Result<Self> {
        let mut options = Self::default();
        let mut retained = Vec::with_capacity(attrs.len());

        for attribute in attrs.drain(..) {
            if !Self::configures(&attribute) {
                retained.push(attribute);
                continue;
            }
            if matches!(attribute.meta, Meta::Path(_)) {
                continue;
            }
            attribute.parse_args_with(|input: ParseStream| options.parse_entries(input))?;
        }

        *attrs = retained;
        Ok(options)
    }

    /// Collects the comma-separated entries of one `#[strutuct(...)]` attribute.
    ///
    /// A visibility entry is the inherited default for the whole branch below the
    /// object carrying the attribute; the remaining entries are named flags.
    fn parse_entries(&mut self, input: ParseStream) -> syn::Result<()> {
        while !input.is_empty() {
            let entry = input.span();
            if peeks_visibility(input) {
                if self
                    .visibility
                    .replace(parse_explicit_visibility(input)?)
                    .is_some()
                {
                    return Err(Error::new(entry, "duplicate `strutuct` visibility"));
                }
            } else {
                let name = input.parse::<Ident>()?;
                let spelling = name.to_string();
                let (option, enabled) = match spelling.strip_prefix("no_") {
                    Some(option) => (option, false),
                    None => (spelling.as_str(), true),
                };
                let option = match option {
                    "inc" => "inclusions",
                    "prod" => "product_variants",
                    "rev" => "reverse_concat",
                    "undoc" => "allow_missing_docs",
                    option => option,
                };
                let slot = match option {
                    "inclusions" => &mut self.inclusions,
                    "product_variants" => &mut self.product_variants,
                    "reverse_concat" => &mut self.reverse_concat,
                    "allow_missing_docs" => &mut self.allow_missing_docs,
                    _ => {
                        return Err(Error::new(
                            entry,
                            "unknown `strutuct` option; expected a visibility, `inclusions` (`inc`), \
                             `product_variants` (`prod`), `reverse_concat` (`rev`), \
                             `allow_missing_docs` (`undoc`), or a `no_` form of these flags",
                        ));
                    }
                };
                if input.peek(Token![=]) {
                    return Err(Error::new(
                        entry,
                        format!(
                            "use the bare flag `{option}` or `no_{option}` instead of a boolean value"
                        ),
                    ));
                }
                if slot.is_some() {
                    return Err(Error::new(entry, "duplicate `strutuct` option"));
                }
                *slot = Some(enabled);
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(())
    }
}

/// Reports whether an explicitly written visibility, including `priv`, comes next.
fn peeks_visibility(input: ParseStream) -> bool {
    input.peek(Token![pub]) || input.peek(Token![priv])
}

/// Parses `pub`, a restricted `pub(...)`, or `priv` as the explicit private counterpart.
fn parse_explicit_visibility(input: ParseStream) -> syn::Result<Visibility> {
    if input.parse::<Option<Token![priv]>>()?.is_some() {
        return Ok(Visibility::Inherited);
    }
    input.parse()
}

#[cfg(test)]
mod argument_tests {
    //! Validation shared by root envelopes and declaration-local options.

    use super::OptionOverrides;

    /// Full and short names resolve to one option, including negated overrides.
    #[test]
    fn aliases_share_values_and_duplicate_checks() {
        let enabled: OptionOverrides = syn::parse_str("inc, prod, rev, undoc").unwrap();
        assert_eq!(enabled.inclusions, Some(true));
        assert_eq!(enabled.product_variants, Some(true));
        assert_eq!(enabled.reverse_concat, Some(true));
        assert_eq!(enabled.allow_missing_docs, Some(true));

        let disabled = enabled.with(syn::parse_str("no_inc, no_prod, no_rev, no_undoc").unwrap());
        assert_eq!(disabled.inclusions, Some(false));
        assert_eq!(disabled.product_variants, Some(false));
        assert_eq!(disabled.reverse_concat, Some(false));
        assert_eq!(disabled.allow_missing_docs, Some(false));

        for (name, alias) in [
            ("inclusions", "inc"),
            ("product_variants", "prod"),
            ("reverse_concat", "rev"),
            ("allow_missing_docs", "undoc"),
        ] {
            for options in [
                format!("{name}, {alias}"),
                format!("no_{alias}, {name}"),
                format!("no_{name}, {alias}"),
            ] {
                let error = syn::parse_str::<OptionOverrides>(&options).err().unwrap();
                assert!(
                    error.to_string().contains("duplicate"),
                    "{options}: {error}"
                );
            }
            for options in [format!("{alias} = true"), format!("no_{alias} = false")] {
                let error = syn::parse_str::<OptionOverrides>(&options).err().unwrap();
                assert!(error.to_string().contains("use the bare flag"));
            }
        }
    }
}

/// Expands one nested algebraic declaration into ordinary Rust items.
pub fn strutuct(input: TokenStream) -> TokenStream {
    parse2::<Invocation>(input)
        .and_then(Invocation::lower)
        .unwrap_or_else(Error::into_compile_error)
}

/// Lowering behavior in effect for one declaration, field, or variant branch.
///
/// Every field corresponds to one `#[strutuct(...)]` entry; the defaults equal
///
/// ```
/// #[strutuct(no_inclusions, no_product_variants, priv, no_reverse_concat)]
/// ```
#[derive(Clone)]
struct Options {
    /// Whether enums generate consuming inclusion functions for constructor paths.
    inclusions: bool,
    /// Whether tuple-like and struct-like variants carry one packed product payload.
    product_variants: bool,
    /// Visibility of every generated declaration and field written without one.
    visibility: Visibility,
    /// Order in which generated names are concatenated.
    concat_order: ConcatOrder,
    /// Whether missing documentation is allowed on generated declarations and members.
    allow_missing_docs: bool,
}

/// Supplies the default algebraic lowering behavior.
impl Default for Options {
    /// Emits private items and ordinary Rust variants, without inclusions.
    fn default() -> Self {
        Self {
            inclusions: false,
            product_variants: false,
            visibility: Visibility::Inherited,
            concat_order: ConcatOrder::Written,
            allow_missing_docs: false,
        }
    }
}

impl Options {
    /// Returns a copy updated by every option explicitly set on one object.
    fn with(&self, overrides: &OptionOverrides) -> Self {
        Self {
            inclusions: overrides.inclusions.unwrap_or(self.inclusions),
            product_variants: overrides.product_variants.unwrap_or(self.product_variants),
            allow_missing_docs: overrides
                .allow_missing_docs
                .unwrap_or(self.allow_missing_docs),
            visibility: overrides
                .visibility
                .clone()
                .unwrap_or_else(|| self.visibility.clone()),
            concat_order: overrides
                .reverse_concat
                .map_or(self.concat_order, ConcatOrder::from),
        }
    }

    /// Resolves the visibility written before one object against the inherited default.
    fn resolve_visibility(&self, explicit: Option<Visibility>) -> Visibility {
        explicit.unwrap_or_else(|| self.visibility.clone())
    }

    /// Applies the optional documentation lint allowance to one generated syntax node.
    fn documentation_lint(&self) -> Option<Attribute> {
        self.allow_missing_docs
            .then(|| syn::parse_quote!(#[allow(missing_docs)]))
    }
}

/// Order in which two identifiers are joined into one generated name.
///
/// ```rust,ignore
/// Root field: Child { A }                                  // RootChild
/// #[strutuct(reverse_concat)] Root field: Child { A } // ChildRoot
/// ```
#[derive(Clone, Copy)]
enum ConcatOrder {
    /// Joins the identifiers in the order they are given.
    Written,
    /// Joins the identifiers in the opposite order.
    Reversed,
}

/// Selects the order from the value of the `reverse_concat` option.
impl From<bool> for ConcatOrder {
    /// Maps `reverse_concat` to [`ConcatOrder::Reversed`].
    fn from(reversed: bool) -> Self {
        if reversed {
            Self::Reversed
        } else {
            Self::Written
        }
    }
}

impl ConcatOrder {
    /// Concatenates two identifiers, spanning the result at the trailing one.
    fn join(self, first: &Ident, second: &Ident) -> Ident {
        let (leading, trailing) = match self {
            Self::Written => (first, second),
            Self::Reversed => (second, first),
        };
        Ident::new(
            &format!("{}{}", leading.unraw(), trailing.unraw()),
            trailing.span(),
        )
    }
}

/// Role an outer attribute plays when it is forwarded to generated items.
///
/// ```rust,ignore
/// #[doc = "..."]       Documentation: stays on the item it was written on
/// #[derive(Debug)]     Derive:        merged into every generated descendant
/// #[underive(Debug)]   Underive:      subtracted from one generated branch
/// #[cfg(test)]         Conditional:   also guards the constructor macros
/// #[repr(C)]           Other:         copied verbatim
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
enum AttributeRole {
    /// Documentation local to one generated item.
    Documentation,
    /// Traits derived by a generated declaration and its descendants.
    Derive,
    /// Traits removed from one generated branch.
    Underive,
    /// Conditional compilation deciding whether a generated item exists.
    Conditional,
    /// Any other attribute, forwarded unchanged.
    Other,
}

impl AttributeRole {
    /// Classifies one attribute by its path.
    fn of(attribute: &Attribute) -> Self {
        let path = attribute.path();
        if path.is_ident("doc") {
            Self::Documentation
        } else if path.is_ident("derive") {
            Self::Derive
        } else if path.is_ident("underive") {
            Self::Underive
        } else if path.is_ident("cfg") || path.is_ident("cfg_attr") {
            Self::Conditional
        } else {
            Self::Other
        }
    }

    /// Reports whether the attribute changes the derived trait list.
    fn modifies_derives(self) -> bool {
        matches!(self, Self::Derive | Self::Underive)
    }

    /// Clones every attribute of this role.
    fn select(self, attrs: &[Attribute]) -> Vec<Attribute> {
        attrs
            .iter()
            .filter(|attribute| Self::of(attribute) == self)
            .cloned()
            .collect()
    }
}

/// Collections of syntax nodes deduplicated by their token spelling.
///
/// ```rust,ignore
/// [Debug, Clone].push_unique(Clone)   // [Debug, Clone]
/// ```
trait UniqueTokens<T> {
    /// Reports whether an identically spelled item is already present.
    fn contains_tokens(&self, item: &T) -> bool;

    /// Appends every item whose spelling is not yet present.
    fn extend_unique(&mut self, items: impl IntoIterator<Item = T>);
}

impl<T: ToTokens> UniqueTokens<T> for Vec<T> {
    /// Compares items by their normalized token spelling.
    fn contains_tokens(&self, item: &T) -> bool {
        let spelling = item.to_token_stream().to_string();
        self.iter()
            .any(|existing| existing.to_token_stream().to_string() == spelling)
    }

    /// Keeps the first occurrence of every spelling.
    fn extend_unique(&mut self, items: impl IntoIterator<Item = T>) {
        for item in items {
            if !self.contains_tokens(&item) {
                self.push(item);
            }
        }
    }
}

/// Trait paths merged from `derive` attributes and reduced by `underive` attributes.
///
/// ```rust,ignore
/// #[derive(Debug, Clone)] Root #[underive(Clone)] child: Child { A, B },
/// // Root derives Debug and Clone; RootChild derives Debug only
/// ```
#[derive(Default)]
struct DeriveList {
    /// Derived trait paths without duplicates, in first-seen order.
    paths: Vec<Path>,
}

impl DeriveList {
    /// Parses the trait paths listed by one `derive` or `underive` attribute.
    fn paths_of(attribute: &Attribute) -> syn::Result<Punctuated<Path, Token![,]>> {
        attribute.parse_args_with(Punctuated::parse_terminated)
    }

    /// Adds the traits of one `derive` attribute.
    fn add(&mut self, attribute: &Attribute) -> syn::Result<()> {
        self.paths.extend_unique(Self::paths_of(attribute)?);
        Ok(())
    }

    /// Removes the traits of one `underive` attribute, ignoring absent ones.
    fn subtract(&mut self, attribute: &Attribute) -> syn::Result<()> {
        let removals = Self::paths_of(attribute)?.into_iter().collect::<Vec<_>>();
        self.paths.retain(|path| !removals.contains_tokens(path));
        Ok(())
    }

    /// Emits one combined `derive` attribute unless no trait remains.
    fn into_attribute(self, span: Span) -> Option<Attribute> {
        let paths = self.paths;
        (!paths.is_empty()).then(|| parse_quote_spanned!(span=> #[derive(#(#paths),*)]))
    }

    /// Moves derive modifiers written on a field or variant to its generated declarations.
    ///
    /// Returns the remaining attributes, which stay on the field or variant itself.
    fn distribute(
        attrs: Vec<Attribute>,
        mut nested: Vec<&mut Declaration>,
        missing_target: &str,
    ) -> syn::Result<Vec<Attribute>> {
        let (modifiers, retained): (Vec<_>, Vec<_>) = attrs
            .into_iter()
            .partition(|attribute| AttributeRole::of(attribute).modifies_derives());
        if nested.is_empty() {
            return match modifiers.first() {
                Some(modifier) => Err(Error::new_spanned(modifier, missing_target)),
                None => Ok(retained),
            };
        }
        for declaration in &mut nested {
            declaration.attrs.extend(modifiers.iter().cloned());
        }
        Ok(retained)
    }
}

/// One complete invocation: forwarded attributes followed by the root declaration.
///
/// ```rust,ignore
/// #[derive(Debug)] ; pub Request method: Method { Get, Post }, body: String?,
/// pub struct Request { method: Method { Get, Post }, body: String? }
/// Pair(String, u8)
/// ```
struct Invocation {
    /// Root algebraic declaration.
    declaration: Declaration,
}

/// Parses forwarded attributes, the root header, its name, and its body.
impl Parse for Invocation {
    /// Treats attributes before an optional `;` as defaults the root header may override.
    ///
    /// The body is braced, parenthesized as the whole remaining input, or written bare.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let parameters = crate::helpers::arguments::input::parse_prefix::<OptionOverrides>(input)?
            .unwrap_or_default();
        let mut forwarded = input.call(Attribute::parse_outer)?;
        let forwarded_options = parameters.with(OptionOverrides::take_from(&mut forwarded)?);
        input.parse::<Option<Token![;]>>()?;
        let mut header = input.parse::<DeclarationHeader>()?;
        header.attrs.splice(0..0, forwarded);
        header.options = forwarded_options.with(header.options);
        let ident = Ident::parse_any(input)?;

        let rest = input.fork();
        let parenthesized_only =
            rest.peek(Paren) && rest.parse::<TokenTree>().is_ok() && rest.is_empty();
        let declaration = if input.peek(Brace) || parenthesized_only {
            let declaration =
                Declaration::parse_delimited(header, ident, DeclarationName::Exact, input)?;
            if !input.is_empty() {
                return Err(input.error("unexpected tokens after braced root declaration"));
            }
            declaration
        } else {
            Declaration::from_body(header, ident, DeclarationName::Exact, input)?
        };
        Ok(Self { declaration })
    }
}

impl Invocation {
    /// Propagates the root attributes and lowers the whole family into Rust items.
    fn lower(self) -> syn::Result<TokenStream> {
        let mut declaration = self.declaration;
        declaration.inherit_attrs(&[])?;
        let lowered = declaration.lower(&Options::default(), false)?;
        Ok(lowered.items.into_iter().collect())
    }
}

/// Attributes, options, visibility, and shape keyword written before a member.
///
/// ```rust,ignore
/// #[derive(Debug)] #[strutuct(pub(crate))] pub(crate) enum   Choice { A, B }
/// #[serde(default)] priv                                       field: u8
/// ```
#[derive(Default)]
struct DeclarationHeader {
    /// Ordinary Rust attributes, with configuration attributes removed.
    attrs: Vec<Attribute>,
    /// Configuration taken from `#[strutuct(...)]` attributes.
    options: OptionOverrides,
    /// Explicit Rust visibility, with inherited visibility representing `priv`.
    visibility: Option<Visibility>,
    /// Optional `struct` or `enum` keyword checked against the inferred shape.
    keyword: Option<DeclarationKeyword>,
}

/// Parses each part of the header, all of which are optional.
impl Parse for DeclarationHeader {
    /// Accepts `priv` in addition to Rust visibilities to request private items.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = input.call(Attribute::parse_outer)?;
        let options = OptionOverrides::take_from(&mut attrs)?;
        let visibility = peeks_visibility(input)
            .then(|| parse_explicit_visibility(input))
            .transpose()?;
        let keyword = if let Some(token) = input.parse::<Option<Token![struct]>>()? {
            Some(DeclarationKeyword::Struct(token.span))
        } else {
            input
                .parse::<Option<Token![enum]>>()?
                .map(|token| DeclarationKeyword::Enum(token.span))
        };
        Ok(Self {
            attrs,
            options,
            visibility,
            keyword,
        })
    }
}

/// Optional Rust-like keyword used only to validate an inferred declaration shape.
///
/// ```rust,ignore
/// struct Point { x: f32, y: f32 }   // accepted
/// struct Choice { A, B }            // rejected: the body is an enum
/// ```
#[derive(Clone, Copy)]
enum DeclarationKeyword {
    /// Requires an inferred unit, named-field, or tuple struct.
    Struct(Span),
    /// Requires an inferred enum.
    Enum(Span),
}

impl DeclarationKeyword {
    /// Returns the location of the keyword.
    fn span(self) -> Span {
        match self {
            Self::Struct(span) | Self::Enum(span) => span,
        }
    }

    /// Rejects a keyword that disagrees with the shape inferred from the body.
    fn validate(self, body: &DeclarationBody) -> syn::Result<()> {
        let (expected, inferred) = match (self, body) {
            (Self::Struct(_), DeclarationBody::Enum(_)) => ("struct", "enum"),
            (Self::Enum(_), body) if !matches!(body, DeclarationBody::Enum(_)) => {
                ("enum", "struct")
            }
            _ => return Ok(()),
        };
        Err(Error::new(
            self.span(),
            format!("`{expected}` does not match the inferred {inferred} declaration"),
        ))
    }
}

/// One declaration whose shape is inferred from its members.
///
/// ```rust,ignore
/// Request { method: Method, body: String }   // struct
/// Pair(u8, u8)                               // tuple struct
/// Method { Get, Post }                       // enum
/// |Marker|                                   // unit struct
/// ```
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

/// Parses an inline declaration together with its own header.
impl Parse for Declaration {
    /// Parses `Name { ... }`, `Name(...)`, `|Name| { ... }`, `|Name|(...)`, or `|Name|`
    /// after the header.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let header = input.parse()?;
        Self::parse_after(header, input)
    }
}

impl Declaration {
    /// Reports whether an inline declaration rather than a Rust type comes next.
    ///
    /// A parenthesized relative name must not be a keyword, which keeps `fn(u8)` a type.
    fn begins(input: ParseStream) -> bool {
        let fork = input.fork();
        if fork.parse::<DeclarationHeader>().is_err() {
            return false;
        }
        if fork.peek(Token![|]) {
            return true;
        }
        if fork.peek2(Brace) {
            return Ident::parse_any(&fork).is_ok();
        }
        fork.parse::<Ident>().is_ok() && fork.peek(Paren)
    }

    /// Assembles a declaration after checking its keyword against the body shape.
    fn new(
        header: DeclarationHeader,
        ident: Ident,
        name: DeclarationName,
        body: DeclarationBody,
    ) -> syn::Result<Self> {
        if let Some(keyword) = header.keyword {
            keyword.validate(&body)?;
        }
        Ok(Self {
            attrs: header.attrs,
            options: header.options,
            visibility: header.visibility,
            ident,
            name,
            body,
        })
    }

    /// Parses a nonempty body whose shape determines the declaration kind.
    fn from_body(
        header: DeclarationHeader,
        ident: Ident,
        name: DeclarationName,
        body: ParseStream,
    ) -> syn::Result<Self> {
        if body.is_empty() {
            return Err(Error::new(
                ident.span(),
                "an empty declaration is ambiguous; add at least one field or variant",
            ));
        }
        Self::new(header, ident, name, body.parse()?)
    }

    /// Parses a braced struct or enum body, or a parenthesized tuple struct body.
    fn parse_delimited(
        header: DeclarationHeader,
        ident: Ident,
        name: DeclarationName,
        input: ParseStream,
    ) -> syn::Result<Self> {
        if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            let fields = Punctuated::<TupleField, Token![,]>::parse_terminated(&content)?;
            return Self::new(
                header,
                ident,
                name,
                DeclarationBody::Tuple(fields.into_iter().collect()),
            );
        }
        let content;
        braced!(content in input);
        Self::from_body(header, ident, name, &content)
    }

    /// Parses the name and body of an inline declaration whose header is already known.
    fn parse_after(header: DeclarationHeader, input: ParseStream) -> syn::Result<Self> {
        if input.parse::<Option<Token![|]>>()?.is_none() {
            let ident = Ident::parse_any(input)?;
            return Self::parse_delimited(header, ident, DeclarationName::Relative, input);
        }

        let ident = Ident::parse_any(input)?;
        input.parse::<Token![|]>()?;
        if input.peek(Brace) || input.peek(Paren) {
            Self::parse_delimited(header, ident, DeclarationName::Exact, input)
        } else {
            Self::new(header, ident, DeclarationName::Exact, DeclarationBody::Unit)
        }
    }

    /// Merges inherited attributes into this declaration and, recursively, its descendants.
    ///
    /// Derives are merged and reduced by local `underive` lists; documentation stays local.
    fn inherit_attrs(&mut self, inherited: &[Attribute]) -> syn::Result<()> {
        let derive_span = self
            .attrs
            .iter()
            .chain(inherited)
            .find(|attribute| AttributeRole::of(attribute) == AttributeRole::Derive)
            .map_or_else(Span::call_site, Attribute::span);
        let mut derives = DeriveList::default();
        for attribute in inherited.iter().chain(&self.attrs) {
            if AttributeRole::of(attribute) == AttributeRole::Derive {
                derives.add(attribute)?;
            }
        }
        for attribute in &self.attrs {
            if AttributeRole::of(attribute) == AttributeRole::Underive {
                derives.subtract(attribute)?;
            }
        }

        self.attrs
            .retain(|attribute| !AttributeRole::of(attribute).modifies_derives());
        if let Some(derive) = derives.into_attribute(derive_span) {
            self.attrs.insert(0, derive);
        }
        for attribute in inherited.iter().rev() {
            let role = AttributeRole::of(attribute);
            if matches!(role, AttributeRole::Conditional | AttributeRole::Other)
                && !self.attrs.contains_tokens(attribute)
            {
                self.attrs.insert(0, attribute.clone());
            }
        }

        let descendants = self
            .attrs
            .iter()
            .filter(|attribute| AttributeRole::of(attribute) != AttributeRole::Documentation)
            .cloned()
            .collect::<Vec<_>>();
        for declaration in self.body.nested_declarations_mut() {
            declaration.inherit_attrs(&descendants)?;
        }
        Ok(())
    }

    /// Lowers this declaration after recursively lowering all nested declarations.
    fn lower(
        self,
        options: &Options,
        inclusion_owner_active: bool,
    ) -> syn::Result<LoweredDeclaration> {
        let options = options.with(&self.options);
        let mut attrs = self.attrs;
        attrs.extend(options.documentation_lint());
        let item = GeneratedItem {
            attrs,
            visibility: options.resolve_visibility(self.visibility),
            ident: self.ident,
        };

        match self.body {
            DeclarationBody::Unit => Ok(item.lower_unit()),
            DeclarationBody::Struct(fields) => item.lower_struct(fields, &options),
            DeclarationBody::Tuple(fields) => item.lower_tuple(fields, &options),
            DeclarationBody::Enum(variants) => {
                item.lower_enum(variants, &options, inclusion_owner_active)
            }
        }
    }

    /// Resolves a relative name against its generated parent before lowering.
    fn lower_nested(
        mut self,
        parent: &Ident,
        options: &Options,
        inclusion_owner_active: bool,
    ) -> syn::Result<LoweredDeclaration> {
        if matches!(self.name, DeclarationName::Relative) {
            let order = options.with(&self.options).concat_order;
            self.ident = order.join(parent, &self.ident);
            self.name = DeclarationName::Exact;
        }
        self.lower(options, inclusion_owner_active)
    }
}

/// How an inline declaration's identifier becomes its generated Rust name.
///
/// ```rust,ignore
/// Root field: Child { A }    // Relative: RootChild
/// Root field: |Child| { A }  // Exact:    Child
/// ```
#[derive(Clone, Copy)]
enum DeclarationName {
    /// Keeps the identifier exactly as written, as for roots and `|Type|`.
    Exact,
    /// Concatenates the identifier with its generated parent declaration.
    Relative,
}

/// The declaration shapes inferred from the DSL body.
///
/// ```rust,ignore
/// |Marker|                  // Unit: no body
/// (String, u8)              // Tuple: a parenthesized body
/// { x: f32, y: f32 }        // Struct: a braced body whose members start with `name:`
/// { A, B(u8), C { x: u8 } } // Enum: any other braced body
/// ```
enum DeclarationBody {
    /// A nominal unit type declared without a body.
    Unit,
    /// A product type recognized by leading `name: Type` members.
    Struct(Vec<StructField>),
    /// A tuple struct declared by a parenthesized body.
    Tuple(Vec<TupleField>),
    /// A sum type recognized by variant-shaped members.
    Enum(Vec<ParsedEnumVariant>),
}

/// Infers the shape of a braced body from its first member.
impl Parse for DeclarationBody {
    /// Separates variants with optional commas, which delimited variants do not need.
    ///
    /// A braced body never holds a bare type, so `Name { ... }` and `Name(...)` members
    /// are variants here rather than nested declarations.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if StructField::begins(input) {
            let fields = Punctuated::<StructField, Token![,]>::parse_terminated(input)?;
            return Ok(Self::Struct(fields.into_iter().collect()));
        }

        let mut variants = Vec::new();
        while !input.is_empty() {
            variants.push(input.parse()?);
            input.parse::<Option<Token![,]>>()?;
        }
        Ok(Self::Enum(variants))
    }
}

impl DeclarationBody {
    /// Returns every declaration generated directly inside this body.
    fn nested_declarations_mut(&mut self) -> Vec<&mut Declaration> {
        match self {
            Self::Unit => Vec::new(),
            Self::Struct(fields) => fields
                .iter_mut()
                .flat_map(|field| field.ty.nested_declarations_mut())
                .collect(),
            Self::Tuple(fields) => fields
                .iter_mut()
                .flat_map(|field| field.ty.nested_declarations_mut())
                .collect(),
            Self::Enum(variants) => variants
                .iter_mut()
                .flat_map(|variant| variant.kind.declarations_mut())
                .collect(),
        }
    }
}

/// One named field of a generated struct or struct-like variant.
///
/// ```rust,ignore
/// #[serde(default)] priv method: Method { Get, Post }?
/// ```
struct StructField {
    attrs: Vec<Attribute>,
    options: OptionOverrides,
    visibility: Option<Visibility>,
    ident: Ident,
    ty: TypeExpression,
}

/// Parses `name: Type`, moving derive modifiers onto the generated field type.
impl Parse for StructField {
    /// Rejects enum-shaped members, which cannot appear among struct fields.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if !Self::begins(input) {
            return Err(input.error("expected a struct field in the form `name: Type`"));
        }
        let header = input.parse::<DeclarationHeader>()?;
        let ident = Ident::parse_any(input)?;
        input.parse::<Token![:]>()?;
        let mut ty = input.parse::<TypeExpression>()?;
        let attrs = DeriveList::distribute(
            header.attrs,
            ty.nested_declarations_mut(),
            "a local derive modifier requires an inline generated field type",
        )?;

        Ok(Self {
            attrs,
            options: header.options,
            visibility: header.visibility,
            ident,
            ty,
        })
    }
}

impl StructField {
    /// Reports whether the next member has the `name: Type` shape.
    fn begins(input: ParseStream) -> bool {
        let fork = input.fork();
        fork.parse::<DeclarationHeader>()
            .is_ok_and(|header| header.keyword.is_none())
            && Ident::parse_any(&fork).is_ok()
            && fork.peek(Token![:])
    }
}

/// One positional field of a generated tuple struct.
///
/// ```rust,ignore
/// #[serde(default)] priv Method { Get, Post }?
/// ```
struct TupleField {
    attrs: Vec<Attribute>,
    options: OptionOverrides,
    visibility: Option<Visibility>,
    ty: TypeExpression,
}

/// Parses an optionally attributed type, moving derive modifiers onto the generated field type.
impl Parse for TupleField {
    /// Hands a shape keyword written in the field header to the declaration that follows.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let DeclarationHeader {
            attrs,
            options,
            visibility,
            keyword,
        } = input.parse()?;
        let mut ty = match keyword {
            Some(keyword) => {
                let header = DeclarationHeader {
                    keyword: Some(keyword),
                    ..DeclarationHeader::default()
                };
                TypeExpression::declared_after(header, input)?
            }
            None => input.parse()?,
        };
        let attrs = DeriveList::distribute(
            attrs,
            ty.nested_declarations_mut(),
            "a local derive modifier requires an inline generated field type",
        )?;

        Ok(Self {
            attrs,
            options,
            visibility,
            ty,
        })
    }
}

/// One enum variant together with its attributes and local options.
///
/// ```rust,ignore
/// #[strutuct(inclusions)] Branch(Branch { Leaf(String), Empty })
/// ```
struct ParsedEnumVariant {
    /// Ordinary Rust attributes forwarded to the generated variant.
    attrs: Vec<Attribute>,
    /// Configuration applied to this variant and its generated payload branch.
    options: OptionOverrides,
    /// Syntactic form of the variant.
    kind: EnumVariant,
}

/// Parses one variant in any of the forms listed by [`EnumVariant`].
impl Parse for ParsedEnumVariant {
    /// Applies the header's keyword to the generated payload, if any.
    ///
    /// As in Rust, a variant takes no visibility; only a leading `|Payload|`, whose
    /// header is the payload declaration's own, accepts one.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start = input.span();
        let DeclarationHeader {
            attrs,
            options,
            visibility,
            mut keyword,
        } = input.parse()?;

        let mut kind = if input.peek(Token![|]) {
            let header = DeclarationHeader {
                visibility,
                keyword: keyword.take(),
                ..DeclarationHeader::default()
            };
            EnumVariant::Implicit(Box::new(TypeExpression::declared_after(header, input)?))
        } else if visibility.is_some() {
            return Err(Error::new(
                start,
                "a variant takes no visibility; write it before the generated payload type, \
                 as in `Name(pub Payload { ... })`",
            ));
        } else if input.peek(Paren) {
            let payload = input.parse::<ParenthesizedTypes>()?;
            if payload.types.len() != 1 {
                return Err(Error::new(
                    payload.paren.span.join(),
                    "an implicit variant accepts exactly one type",
                ));
            }
            if input.peek(Brace) {
                return Err(
                    input.error("use `|Type| { ... }` to define an explicitly named payload type")
                );
            }
            let ty = payload
                .types
                .into_iter()
                .next()
                .expect("checked one type above");
            EnumVariant::Implicit(Box::new(ty))
        } else {
            let ident = Ident::parse_any(input)?;
            if input.peek(Token![|]) {
                let header = DeclarationHeader {
                    keyword: keyword.take(),
                    ..DeclarationHeader::default()
                };
                EnumVariant::Tuple {
                    ident,
                    fields: vec![TypeExpression::declared_after(header, input)?],
                }
            } else if input.peek(Brace) {
                let content;
                braced!(content in input);
                let fields = Punctuated::<StructField, Token![,]>::parse_terminated(&content)?;
                let header = DeclarationHeader {
                    keyword: keyword.take(),
                    ..DeclarationHeader::default()
                };
                EnumVariant::Struct {
                    ident: ident.clone(),
                    declaration: Box::new(Declaration::new(
                        header,
                        ident,
                        DeclarationName::Relative,
                        DeclarationBody::Struct(fields.into_iter().collect()),
                    )?),
                }
            } else if input.peek(Paren) {
                let payload = input.parse::<ParenthesizedTypes>()?;
                if input.peek(Brace) {
                    return Err(input.error(
                        "use `Name |Type| { ... }` to name an explicitly generated payload type",
                    ));
                }
                EnumVariant::Tuple {
                    ident,
                    fields: payload.types.into_iter().collect(),
                }
            } else {
                EnumVariant::Unit(ident)
            }
        };

        if let Some(keyword) = keyword {
            return Err(Error::new(
                keyword.span(),
                "`struct` and `enum` can annotate only an inline generated declaration here",
            ));
        }
        let attrs = DeriveList::distribute(
            attrs,
            kind.declarations_mut(),
            "a local derive modifier requires an inline generated payload type",
        )?;

        Ok(Self {
            attrs,
            options,
            kind,
        })
    }
}

/// Syntactic form of one generated enum variant.
///
/// ```rust,ignore
/// Null                   // Unit:     Null
/// Pair(String, u8)       // Tuple:    Pair(String, u8)
/// Span { start: usize }  // Struct:   Span { start: usize }
/// Named |Payload|        // Tuple:    Named(Payload)
/// (Bin)                  // Implicit: BinExpr(Bin)
/// |Payload| { A, B }     // Implicit: PayloadExpr(Payload)
/// ```
///
/// With `product_variants`, `Pair` becomes `Pair((String, u8))` and `Span`
/// becomes `Span(ExprSpan)` with a generated `ExprSpan` struct.
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
    /// An explicitly named struct-like variant.
    Struct {
        /// Variant name retained verbatim.
        ident: Ident,
        /// Fields held as a struct declaration, which becomes the payload struct
        /// under `product_variants` and passes inherited attributes to nested types.
        declaration: Box<Declaration>,
    },
    /// An explicitly named unit variant.
    Unit(Ident),
}

impl EnumVariant {
    /// Returns declarations generated directly by this variant's payload.
    fn declarations_mut(&mut self) -> Vec<&mut Declaration> {
        match self {
            Self::Implicit(ty) => ty.nested_declarations_mut(),
            Self::Tuple { fields, .. } => fields
                .iter_mut()
                .flat_map(TypeExpression::nested_declarations_mut)
                .collect(),
            Self::Struct { declaration, .. } => vec![declaration],
            Self::Unit(_) => Vec::new(),
        }
    }
}

/// A parenthesized, comma-separated list of type expressions.
///
/// ```rust,ignore
/// (String, Item { A, B }?)
/// ```
struct ParenthesizedTypes {
    /// The enclosing parentheses.
    paren: Paren,
    /// Types listed inside the parentheses.
    types: Punctuated<TypeExpression, Token![,]>,
}

/// Parses the parentheses and their optionally comma-terminated contents.
impl Parse for ParenthesizedTypes {
    /// Delegates every element to [`TypeExpression`].
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let paren = parenthesized!(content in input);
        Ok(Self {
            paren,
            types: Punctuated::parse_terminated(&content)?,
        })
    }
}

/// An inline declaration or a Rust type, followed by postfix wrappers.
///
/// Rust types, including their generic arguments, are copied unchanged.
///
/// ```rust,ignore
/// Method { Get, Post }?            // nested enum wrapped in Option
/// Pair(u8, Kind { A, B })          // nested tuple struct containing another declaration
/// Vec<Box<dyn Fn(u8)>>*            // Rust type, boxed
/// ```
struct TypeExpression {
    /// Ordinary Rust type or an inline generated declaration.
    base: TypeBase,
    /// Postfix operators in their source order.
    wrappers: Vec<TypeWrapper>,
}

/// Parses one extended type, leaving the following comma or member untouched.
impl Parse for TypeExpression {
    /// Lets syn determine the extent of a Rust type.
    ///
    /// Rejects `#[strutuct(...)]`, which Rust-like positions put on the enclosing
    /// field, variant, or declaration rather than on a type.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.fork().call(Attribute::parse_outer)?;
        if let Some(configuration) = attrs
            .iter()
            .find(|attribute| OptionOverrides::configures(attribute))
        {
            return Err(Error::new_spanned(
                configuration,
                "`#[strutuct(...)]` cannot configure a type; \
                 write it before the enclosing field, variant, or declaration",
            ));
        }
        let base = if Declaration::begins(input) {
            TypeBase::Nested(Box::new(input.parse()?))
        } else {
            TypeBase::Rust(Box::new(input.parse()?))
        };
        Ok(Self {
            base,
            wrappers: TypeWrapper::parse_postfix(input)?,
        })
    }
}

impl TypeExpression {
    /// Parses an inline declaration whose header was written before its member.
    fn declared_after(header: DeclarationHeader, input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            base: TypeBase::Nested(Box::new(Declaration::parse_after(header, input)?)),
            wrappers: TypeWrapper::parse_postfix(input)?,
        })
    }

    /// Returns the declaration this type generates, if any.
    fn nested_declarations_mut(&mut self) -> Vec<&mut Declaration> {
        match &mut self.base {
            TypeBase::Rust(_) => Vec::new(),
            TypeBase::Nested(declaration) => vec![declaration],
        }
    }

    /// Lowers the type, hoisting a nested declaration and marking wrapped edges as terminal.
    fn lower(
        self,
        options: &Options,
        parent: &Ident,
        inclusion_owner_active: bool,
    ) -> syn::Result<LoweredType> {
        let wrapped = !self.wrappers.is_empty();
        let mut lowered = match self.base {
            TypeBase::Rust(ty) => {
                let name = ty.terminal_ident();
                let ty = ty.into_token_stream();
                LoweredType {
                    name,
                    ty: ty.clone(),
                    items: Vec::new(),
                    algebra: AlgebraicType {
                        ty,
                        shape: AlgebraicShape::Rust,
                        conditional_attrs: Vec::new(),
                        value_constructor: None,
                        declaration_kind: None,
                    },
                    wrapped,
                }
            }
            TypeBase::Nested(declaration) => {
                let declaration = declaration.lower_nested(
                    parent,
                    options,
                    inclusion_owner_active && !wrapped,
                )?;
                LoweredType {
                    ty: declaration.ident.to_token_stream(),
                    name: Some(declaration.ident),
                    items: declaration.items,
                    algebra: declaration.algebra,
                    wrapped,
                }
            }
        };

        for wrapper in self.wrappers {
            let ty = wrapper.wrap(&lowered.ty);
            let inner = lowered.algebra;
            lowered.ty = ty.clone();
            lowered.algebra = AlgebraicType {
                ty,
                conditional_attrs: inner.conditional_attrs.clone(),
                shape: AlgebraicShape::Wrapped {
                    wrapper,
                    inner: Box::new(inner),
                },
                value_constructor: None,
                declaration_kind: None,
            };
        }

        Ok(lowered)
    }
}

/// The unwrapped portion of a field or variant type.
///
/// ```rust,ignore
/// Method { Get, Post }        // Nested
/// Vec<Box<dyn Fn(u8)>>        // Rust
/// ```
enum TypeBase {
    /// An ordinary Rust type, copied unchanged.
    Rust(Box<Type>),
    /// A nested declaration that must be hoisted before its parent.
    Nested(Box<Declaration>),
}

/// Supported postfix type constructors.
///
/// ```rust,ignore
/// Item?    // Option<Item>
/// Item*    // Box<Item>
/// Item?*   // Box<Option<Item>>
/// ```
#[derive(Clone, Copy)]
enum TypeWrapper {
    /// `T?`, lowered to `Option<T>`.
    Option,
    /// `T*`, lowered to `Box<T>`.
    Box,
}

/// Parses one postfix operator.
impl Parse for TypeWrapper {
    /// Accepts `?` or `*`.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.parse::<Option<Token![?]>>()?.is_some() {
            Ok(Self::Option)
        } else {
            input.parse::<Token![*]>()?;
            Ok(Self::Box)
        }
    }
}

impl TypeWrapper {
    /// Parses every postfix operator following a type, in source order.
    fn parse_postfix(input: ParseStream) -> syn::Result<Vec<Self>> {
        let mut wrappers = Vec::new();
        while input.peek(Token![?]) || input.peek(Token![*]) {
            wrappers.push(input.parse()?);
        }
        Ok(wrappers)
    }

    /// Wraps a lowered type in the corresponding standard library type.
    fn wrap(self, inner: &TokenStream) -> TokenStream {
        match self {
            Self::Option => quote!(::core::option::Option<#inner>),
            Self::Box => quote!(::std::boxed::Box<#inner>),
        }
    }
}

/// Final path segment naming a Rust type, used to derive implicit variant names.
///
/// ```rust,ignore
/// std::string::String   // String
/// (Vec<u8>)             // Vec
/// &str                  // none
/// ```
trait TerminalIdent {
    /// Returns the last path segment, looking through parentheses and groups.
    fn terminal_ident(&self) -> Option<Ident>;
}

impl TerminalIdent for Type {
    /// Accepts only unqualified paths.
    fn terminal_ident(&self) -> Option<Ident> {
        match self {
            Type::Path(path) if path.qself.is_none() => path
                .path
                .segments
                .last()
                .map(|segment| segment.ident.clone()),
            Type::Group(group) => group.elem.terminal_ident(),
            Type::Paren(parenthesized) => parenthesized.elem.terminal_ident(),
            _ => None,
        }
    }
}

/// Whether a lowered declaration is a product or sum type, selecting its constructor macro.
///
/// ```rust,ignore
/// |Marker|          // Unit:   Marker!()
/// { x: u8 }         // Struct: Point!(x: 1)
/// (u8, u8)         // Tuple:  Pair!(1, 2)
/// { A, B(u8) }      // Enum:   Choice!(B(1))
/// ```
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

impl DeclarationKind {
    /// Emits one parent-macro arm delegating through a payload of this kind.
    fn constructor_arm(
        self,
        parent: &Ident,
        variant: &Ident,
        selector: &Ident,
        payload: &Ident,
    ) -> TokenStream {
        match self {
            Self::Unit => quote! {
                (#selector) => {
                    #parent::#variant(#payload)
                };
            },
            Self::Enum => quote! {
                (#selector::$($tail:tt)+) => {
                    #parent::#variant(#payload!($($tail)+))
                };
            },
            Self::Struct => quote! {
                (#selector { $($fields:tt)* }) => {
                    #parent::#variant(#payload! { $($fields)* })
                };
            },
            Self::Tuple => quote! {
                (#selector($($fields:tt)*)) => {
                    #parent::#variant(#payload!($($fields)*))
                };
            },
        }
    }

    /// Emits the same-name constructor macro of a generated declaration of this kind.
    ///
    /// Enum macros try the specialized arms delegating into nested payloads first.
    fn constructor_macro(
        self,
        ident: &Ident,
        attrs: &[Attribute],
        specialized_arms: &[TokenStream],
    ) -> TokenStream {
        let article = if matches!(self, Self::Enum) {
            "an"
        } else {
            "a"
        };
        let documentation = LitStr::new(
            &format!("Constructs {article} `{ident}` value."),
            ident.span(),
        );
        match self {
            Self::Unit => quote! {
                #(#attrs)*
                #[doc = #documentation]
                #[allow(unused_macros)]
                macro_rules! #ident {
                    () => {
                        #ident
                    };
                }
            },
            Self::Struct => quote! {
                #(#attrs)*
                #[doc = #documentation]
                #[allow(unused_macros)]
                macro_rules! #ident {
                    ($($fields:tt)*) => {
                        #ident { $($fields)* }
                    };
                }
            },
            Self::Tuple => quote! {
                #(#attrs)*
                #[doc = #documentation]
                #[allow(unused_macros)]
                macro_rules! #ident {
                    ($($fields:tt)*) => {
                        #ident($($fields)*)
                    };
                }
            },
            Self::Enum => quote! {
                #(#attrs)*
                #[doc = #documentation]
                #[allow(unused_macros)]
                macro_rules! #ident {
                    #(#specialized_arms)*
                    ($($variant:tt)+) => {
                        #ident::$($variant)+
                    };
                }
            },
        }
    }
}

/// Resolved attributes, visibility, and name of one Rust item about to be emitted.
///
/// ```rust,ignore
/// #[derive(Debug)] pub struct Request
/// ```
struct GeneratedItem {
    /// Attributes emitted on the item.
    attrs: Vec<Attribute>,
    /// Visibility emitted on the item.
    visibility: Visibility,
    /// Final name of the item.
    ident: Ident,
}

impl GeneratedItem {
    /// Emits a nominal unit struct.
    fn lower_unit(self) -> LoweredDeclaration {
        let Self {
            attrs,
            visibility,
            ident,
        } = self;
        let macro_attrs = AttributeRole::Conditional.select(&attrs);
        let items = vec![
            quote! {
                #[allow(dead_code)]
                #(#attrs)*
                #visibility struct #ident;
            },
            DeclarationKind::Unit.constructor_macro(&ident, &macro_attrs, &[]),
        ];

        LoweredDeclaration {
            items,
            ident: ident.clone(),
            algebra: AlgebraicType {
                ty: ident.to_token_stream(),
                shape: AlgebraicShape::Unit,
                conditional_attrs: macro_attrs,
                value_constructor: Some(ident),
                declaration_kind: Some(DeclarationKind::Unit),
            },
        }
    }

    /// Emits a struct after collecting declarations from every field type.
    fn lower_struct(
        self,
        fields: Vec<StructField>,
        options: &Options,
    ) -> syn::Result<LoweredDeclaration> {
        let Self {
            attrs,
            visibility,
            ident,
        } = self;
        let macro_attrs = AttributeRole::Conditional.select(&attrs);
        let mut items = Vec::new();
        let mut lowered_fields = Vec::with_capacity(fields.len());
        let mut components = Vec::with_capacity(fields.len());

        for field in fields {
            let field_options = options.with(&field.options);
            let lowered = field.ty.lower(&field_options, &ident, false)?;
            items.extend(lowered.items);
            let mut field_attrs = field.attrs;
            field_attrs.extend(field_options.documentation_lint());
            let field_visibility = field_options.resolve_visibility(field.visibility);
            let field_ident = field.ident;
            let field_ty = lowered.ty;
            components.push(ProductComponent {
                selector: ProductSelector::Named(field_ident.clone()),
                ty: lowered.algebra,
            });
            lowered_fields.push(quote! {
                #(#field_attrs)*
                #field_visibility #field_ident: #field_ty
            });
        }

        items.push(quote! {
            #[allow(dead_code)]
            #(#attrs)*
            #visibility struct #ident {
                #(#lowered_fields),*
            }
        });
        items.push(DeclarationKind::Struct.constructor_macro(&ident, &macro_attrs, &[]));

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

    /// Emits a tuple struct after collecting declarations from every field type.
    fn lower_tuple(
        self,
        fields: Vec<TupleField>,
        options: &Options,
    ) -> syn::Result<LoweredDeclaration> {
        let Self {
            attrs,
            visibility,
            ident,
        } = self;
        let macro_attrs = AttributeRole::Conditional.select(&attrs);
        let mut items = Vec::new();
        let mut lowered_fields = Vec::with_capacity(fields.len());
        let mut components = Vec::with_capacity(fields.len());

        for (index, field) in fields.into_iter().enumerate() {
            let field_options = options.with(&field.options);
            let lowered = field.ty.lower(&field_options, &ident, false)?;
            items.extend(lowered.items);
            let mut field_attrs = field.attrs;
            field_attrs.extend(field_options.documentation_lint());
            let field_visibility = field_options.resolve_visibility(field.visibility);
            let field_ty = lowered.ty;
            components.push(ProductComponent {
                selector: ProductSelector::Positional(index),
                ty: lowered.algebra,
            });
            lowered_fields.push(quote! {
                #(#field_attrs)*
                #field_visibility #field_ty
            });
        }

        items.push(quote! {
            #[allow(dead_code)]
            #(#attrs)*
            #visibility struct #ident(#(#lowered_fields),*);
        });
        items.push(DeclarationKind::Tuple.constructor_macro(&ident, &macro_attrs, &[]));

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

    /// Emits an enum and its recursively delegating constructor macro.
    fn lower_enum(
        self,
        variants: Vec<ParsedEnumVariant>,
        options: &Options,
        inclusion_owner_active: bool,
    ) -> syn::Result<LoweredDeclaration> {
        let Self {
            attrs,
            visibility,
            ident,
        } = self;
        let macro_attrs = AttributeRole::Conditional.select(&attrs);
        let mut lowering = EnumLowering {
            ident: ident.clone(),
            items: Vec::new(),
            variants: Vec::with_capacity(variants.len()),
            macro_arms: Vec::new(),
            branches: Vec::with_capacity(variants.len()),
        };
        for variant in variants {
            lowering.push(variant, options)?;
        }
        let EnumLowering {
            mut items,
            variants: lowered_variants,
            macro_arms,
            branches,
            ..
        } = lowering;

        let algebra = AlgebraicType {
            ty: ident.to_token_stream(),
            shape: AlgebraicShape::Coproduct(branches),
            conditional_attrs: macro_attrs.clone(),
            value_constructor: None,
            declaration_kind: Some(DeclarationKind::Enum),
        };
        items.push(quote! {
            #[allow(dead_code, clippy::enum_variant_names)]
            #(#attrs)*
            #visibility enum #ident {
                #(#lowered_variants),*
            }
        });
        items.push(DeclarationKind::Enum.constructor_macro(&ident, &macro_attrs, &macro_arms));
        if !inclusion_owner_active {
            let flattened = algebra.flattened_coproduct();
            debug_assert!(flattened.len() <= algebra.node_count());
            for inclusion in flattened {
                items.push(inclusion.emit(&ident, &visibility)?);
            }
        }

        Ok(LoweredDeclaration {
            items,
            ident,
            algebra,
        })
    }
}

/// Enum variants, hoisted items, macro arms, and algebraic branches collected one variant at a time.
///
/// ```rust,ignore
/// Expr Unary(Unary { (Pref), (Post) }) (Bin) Null
/// // items:      enum ExprUnary and its macro
/// // variants:   Unary(ExprUnary), BinExpr(Bin), Null
/// // macro arms: Unary::... delegating to ExprUnary!, Bin(...) constructing Bin
/// ```
struct EnumLowering {
    /// Name of the enum being lowered.
    ident: Ident,
    /// Nested declarations and macros emitted before the enum.
    items: Vec<TokenStream>,
    /// Lowered Rust variants.
    variants: Vec<TokenStream>,
    /// Constructor-macro arms delegating into generated payloads.
    macro_arms: Vec<TokenStream>,
    /// Algebraic description of every variant.
    branches: Vec<CoproductVariant>,
}

impl EnumLowering {
    /// Lowers one parsed variant under the enum's options.
    fn push(&mut self, variant: ParsedEnumVariant, options: &Options) -> syn::Result<()> {
        let ParsedEnumVariant {
            mut attrs,
            options: overrides,
            kind,
        } = variant;
        let options = options.with(&overrides);
        attrs.extend(options.documentation_lint());
        match kind {
            EnumVariant::Implicit(ty) => self.push_implicit(attrs, *ty, &options),
            EnumVariant::Tuple { ident, fields } => self.push_tuple(attrs, ident, fields, &options),
            EnumVariant::Struct { ident, declaration } => {
                self.push_struct(attrs, ident, *declaration, &options)
            }
            EnumVariant::Unit(ident) => {
                self.push_unit(attrs, ident, &options);
                Ok(())
            }
        }
    }

    /// Emits one Rust variant with its supplied attributes and payload syntax.
    fn emit_variant(&mut self, attrs: &[Attribute], variant: &Ident, payload: TokenStream) {
        self.variants.push(quote! {
            #(#attrs)*
            #variant #payload
        });
    }

    /// Records the algebraic branch of one variant.
    fn push_branch(
        &mut self,
        attrs: &[Attribute],
        selector: Ident,
        variant: Ident,
        payload: AlgebraicType,
        constructor: CoproductConstructor,
        options: &Options,
    ) {
        self.branches.push(CoproductVariant {
            selector,
            variant,
            payload,
            constructor,
            conditional_attrs: AttributeRole::Conditional.select(attrs),
            inclusions: options.inclusions,
            concat_order: options.concat_order,
        });
    }

    /// Lowers `(Type)` or `|Type|`, naming the variant after its payload.
    fn push_implicit(
        &mut self,
        attrs: Vec<Attribute>,
        ty: TypeExpression,
        options: &Options,
    ) -> syn::Result<()> {
        let ident = self.ident.clone();
        let lowered = ty.lower(options, &ident, options.inclusions)?;
        self.items.extend(lowered.items);
        let payload_name = lowered.name.ok_or_else(|| {
            Error::new(
                ident.span(),
                "cannot derive an implicit variant name from this type; name the variant explicitly",
            )
        })?;
        let variant = options.concat_order.join(&payload_name, &ident);
        let payload_ty = lowered.ty;
        self.emit_variant(&attrs, &variant, quote!((#payload_ty)));

        if !lowered.wrapped {
            self.macro_arms
                .push(match lowered.algebra.declaration_kind {
                    Some(kind) => {
                        kind.constructor_arm(&ident, &variant, &payload_name, &payload_name)
                    }
                    None => quote! {
                        (#payload_name($($arguments:tt)*)) => {
                            #ident::#variant(#payload_ty($($arguments)*))
                        };
                        (#payload_name { $($fields:tt)* }) => {
                            #ident::#variant(#payload_ty { $($fields)* })
                        };
                        (#payload_name) => {
                            #ident::#variant(#payload_ty)
                        };
                    },
                });
        }

        self.push_branch(
            &attrs,
            payload_name,
            variant,
            lowered.algebra,
            CoproductConstructor::Unary,
            options,
        );
        Ok(())
    }

    /// Lowers `Name(Type, ...)` or `Name |Type|`, packing several fields into one
    /// product under `product_variants`.
    fn push_tuple(
        &mut self,
        attrs: Vec<Attribute>,
        variant: Ident,
        fields: Vec<TypeExpression>,
        options: &Options,
    ) -> syn::Result<()> {
        let ident = self.ident.clone();
        let composes_nested_enum = options.inclusions && fields.len() == 1;
        let mut fields = fields
            .into_iter()
            .map(|field| field.lower(options, &ident, composes_nested_enum))
            .collect::<syn::Result<Vec<_>>>()?;
        for field in &mut fields {
            self.items.append(&mut field.items);
        }
        if let [field] = fields.as_slice()
            && !field.wrapped
            && let (Some(payload_name), Some(kind)) = (&field.name, field.algebra.declaration_kind)
        {
            self.macro_arms
                .push(kind.constructor_arm(&ident, &variant, &variant, payload_name));
        }

        let product = options.product_variants && fields.len() >= 2;
        let lowered_fields = fields
            .iter()
            .map(|field| field.ty.clone())
            .collect::<Vec<_>>();
        if product {
            self.macro_arms.push(quote! {
                (#variant($($arguments:tt)*)) => {
                    #ident::#variant(($($arguments)*))
                };
            });
            self.emit_variant(&attrs, &variant, quote!(((#(#lowered_fields),*))));
        } else {
            self.emit_variant(&attrs, &variant, quote!((#(#lowered_fields),*)));
        }

        let constructor = if product || fields.len() == 1 {
            CoproductConstructor::Unary
        } else {
            CoproductConstructor::Tuple(fields.len())
        };
        let payload = match fields.len() {
            0 => AlgebraicType::unit(),
            1 => fields.pop().expect("checked one field above").algebra,
            _ => {
                let components = fields
                    .into_iter()
                    .enumerate()
                    .map(|(index, field)| ProductComponent {
                        selector: ProductSelector::Positional(index),
                        ty: field.algebra,
                    })
                    .collect();
                AlgebraicType::anonymous_product(lowered_fields, components)
            }
        };
        self.push_branch(
            &attrs,
            variant.clone(),
            variant,
            payload,
            constructor,
            options,
        );
        Ok(())
    }

    /// Lowers `Name { field: Type, ... }` as an ordinary struct-like variant or,
    /// under `product_variants`, as a variant carrying a generated payload struct.
    fn push_struct(
        &mut self,
        attrs: Vec<Attribute>,
        variant: Ident,
        declaration: Declaration,
        options: &Options,
    ) -> syn::Result<()> {
        let ident = self.ident.clone();
        if !options.product_variants
            && let DeclarationBody::Struct(fields) = declaration.body
        {
            // The declaration becomes a struct-like variant rather than an item.
            // Its attributes have already propagated into generated field types;
            // retargeting them onto the variant would change their syntactic target.
            let mut lowered_fields = Vec::with_capacity(fields.len());
            let mut components = Vec::with_capacity(fields.len());
            let mut field_names = Vec::with_capacity(fields.len());
            for field in fields {
                let field_options = options.with(&field.options);
                let lowered = field.ty.lower(&field_options, &ident, false)?;
                self.items.extend(lowered.items);
                let mut field_attrs = field.attrs;
                field_attrs.extend(field_options.documentation_lint());
                let field_ident = field.ident;
                let field_ty = lowered.ty;
                components.push(ProductComponent {
                    selector: ProductSelector::Named(field_ident.clone()),
                    ty: lowered.algebra,
                });
                field_names.push(field_ident.clone());
                lowered_fields.push(quote! {
                    #(#field_attrs)*
                        #field_ident: #field_ty
                });
            }
            self.emit_variant(&attrs, &variant, quote!({ #(#lowered_fields),* }));
            let payload_types = components
                .iter()
                .map(|component| component.ty.ty.clone())
                .collect();
            self.push_branch(
                &attrs,
                variant.clone(),
                variant,
                AlgebraicType::anonymous_product(payload_types, components),
                CoproductConstructor::Struct(field_names),
                options,
            );
            return Ok(());
        }

        let lowered = declaration.lower_nested(&ident, options, options.inclusions)?;
        self.items.extend(lowered.items);
        let payload_ident = lowered.ident;
        let kind = lowered
            .algebra
            .declaration_kind
            .expect("lowered declarations retain their constructor kind");
        self.macro_arms
            .push(kind.constructor_arm(&ident, &variant, &variant, &payload_ident));
        self.emit_variant(&attrs, &variant, quote!((#payload_ident)));
        self.push_branch(
            &attrs,
            variant.clone(),
            variant,
            lowered.algebra,
            CoproductConstructor::Unary,
            options,
        );
        Ok(())
    }

    /// Lowers a bare `Name` as a unit variant.
    fn push_unit(&mut self, attrs: Vec<Attribute>, variant: Ident, options: &Options) {
        self.emit_variant(&attrs, &variant, TokenStream::new());
        self.push_branch(
            &attrs,
            variant.clone(),
            variant,
            AlgebraicType::unit(),
            CoproductConstructor::Unit,
            options,
        );
    }
}

/// Fully lowered declaration together with its generated items.
///
/// ```rust,ignore
/// Method { Get, Post }   // items: enum RequestMethod, macro RequestMethod; ident: RequestMethod
/// ```
struct LoweredDeclaration {
    /// Generated items ordered with dependencies before their consumers.
    items: Vec<TokenStream>,
    /// Name usable when the declaration is embedded as a type.
    ident: Ident,
    /// Reusable algebraic description of the generated Rust type.
    algebra: AlgebraicType,
}

/// Fully lowered type expression and nested items collected from it.
///
/// ```rust,ignore
/// Method { Get, Post }?   // ty: Option<RequestMethod>, name: RequestMethod, wrapped
/// ```
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

/// A lowered Rust type retaining the algebraic structure relevant to generated maps.
///
/// ```rust,ignore
/// Expr Unary(Unary { (Pref), (Post) }) Null
/// // Coproduct[Unary: Coproduct[Pref: Rust, Post: Rust], Null: Unit]
/// ```
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
///
/// ```rust,ignore
/// Vec<u8>              // Rust
/// Null                 // Unit
/// { x: u8, y: u8 }     // Product
/// { A, B }             // Coproduct
/// Item?                // Wrapped
/// ```
enum AlgebraicShape {
    /// An ordinary Rust type opaque to the algebra.
    Rust,
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
///
/// ```rust,ignore
/// Coproduct[A: Unit, B: Unit] folded by node_count   // Coproduct[(A, 1), (B, 1)]
/// ```
enum FoldedAlgebraicShape<'a, T> {
    /// Ordinary Rust syntax opaque to the algebra.
    Rust,
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
///
/// ```rust,ignore
/// start: usize   // selector: Named(start), ty: usize
/// ```
struct ProductComponent {
    /// Field name or positional index selecting this component.
    selector: ProductSelector,
    /// Algebraic type stored in this component.
    ty: AlgebraicType,
}

/// Projection label belonging to one product component.
///
/// ```rust,ignore
/// Span { start: usize }   // Named(start), projected as .start
/// Pair(u8, u8)            // Positional(0) and Positional(1), projected as .0 and .1
/// ```
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
///
/// ```rust,ignore
/// (Bin) in Expr   // selector: Bin, variant: BinExpr, constructor: Unary
/// ```
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
    /// Concatenation order active on this edge.
    concat_order: ConcatOrder,
}

/// Concrete Rust constructor shape implementing one coproduct injection.
///
/// ```rust,ignore
/// Null               // Unit:      Root::Null
/// Leaf(String)       // Unary:     Root::Leaf(value)
/// Pair(u8, u8)       // Tuple(2):  Root::Pair(value.0, value.1), without product variants
/// Span { start: u8 } // Struct:    Root::Span { start: value.0 }, without product variants
/// ```
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
///
/// ```rust,ignore
/// #[strutuct(inclusions)] A B(B { C })
/// // steps [B, C], no parameter, value A::B(AB::C)   =>   fn ABC() -> A
/// ```
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

/// One named enum edge and the concatenation order active on that edge.
///
/// ```rust,ignore
/// Branch in Root Branch(Branch { Leaf })   // selector: Branch
/// ```
struct InclusionStep {
    /// Selector written for this variant in the `strutuct!` input.
    selector: Ident,
    /// Order in which this edge joins its selector to the accumulated path name.
    concat_order: ConcatOrder,
}

impl AlgebraicType {
    /// Builds the anonymous terminal product used as a unit-variant payload.
    fn unit() -> Self {
        Self {
            ty: quote!(()),
            shape: AlgebraicShape::Unit,
            conditional_attrs: Vec::new(),
            value_constructor: None,
            declaration_kind: None,
        }
    }

    /// Builds an anonymous tuple product from already lowered components.
    fn anonymous_product(types: Vec<TokenStream>, components: Vec<ProductComponent>) -> Self {
        Self {
            ty: quote!((#(#types,)*)),
            shape: AlgebraicShape::Product(components),
            conditional_attrs: Vec::new(),
            value_constructor: None,
            declaration_kind: None,
        }
    }

    /// Folds the complete algebraic tree bottom-up, retaining labels on every edge.
    fn fold<T, F>(&self, algebra: &mut F) -> T
    where
        F: for<'a> FnMut(&'a Self, FoldedAlgebraicShape<'a, T>) -> T,
    {
        let folded = match &self.shape {
            AlgebraicShape::Rust => FoldedAlgebraicShape::Rust,
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
            FoldedAlgebraicShape::Rust | FoldedAlgebraicShape::Unit => 1,
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
        let combined = |left: &[Attribute], right: &[Attribute]| {
            let mut combined = left.to_vec();
            combined.extend_unique(right.iter().cloned());
            combined
        };
        self.fold(
            &mut |node, folded: FoldedAlgebraicShape<'_, Vec<InclusionPath>>| {
                let FoldedAlgebraicShape::Coproduct(variants) = folded else {
                    return Vec::new();
                };
                let mut flattened = Vec::new();

                for (branch, nested_paths) in
                    variants.into_iter().filter(|(branch, _)| branch.inclusions)
                {
                    let step = || InclusionStep {
                        selector: branch.selector.clone(),
                        concat_order: branch.concat_order,
                    };
                    let branch_attrs = combined(&node.conditional_attrs, &branch.conditional_attrs);
                    let value = InclusionPath::argument();
                    flattened.push(InclusionPath {
                        steps: vec![step()],
                        parameter: branch
                            .constructor
                            .takes_value()
                            .then(|| branch.payload.ty.clone()),
                        value: branch.constructor.construct(
                            &node.ty,
                            &branch.variant,
                            Some(value.to_token_stream()),
                        ),
                        conditional_attrs: combined(
                            &branch_attrs,
                            &branch.payload.conditional_attrs,
                        ),
                        value_constructor: branch.payload.value_constructor.clone(),
                    });

                    if !branch.constructor.composes() {
                        continue;
                    }
                    for mut nested in nested_paths {
                        nested.steps.insert(0, step());
                        nested.value = branch.constructor.construct(
                            &node.ty,
                            &branch.variant,
                            Some(nested.value),
                        );
                        nested.conditional_attrs =
                            combined(&branch_attrs, &nested.conditional_attrs);
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

impl InclusionPath {
    /// Returns the hygienic parameter name bound inside generated inclusion functions.
    fn argument() -> Ident {
        Ident::new("value", Span::mixed_site())
    }

    /// Concatenates the root name with every selector on the path.
    fn name(&self, root: &Ident) -> Ident {
        self.steps.iter().fold(root.clone(), |prefix, step| {
            step.concat_order.join(&prefix, &step.selector)
        })
    }

    /// Emits the consuming function injecting this constructor occurrence into the root.
    fn emit(self, root: &Ident, visibility: &Visibility) -> syn::Result<TokenStream> {
        let ident = self.name(root);
        if self
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

        let documentation = LitStr::new(
            &format!(
                "Injects the `{}` constructor occurrence into `{root}`.",
                self.steps
                    .iter()
                    .map(|step| step.selector.unraw().to_string())
                    .collect::<Vec<_>>()
                    .join("::")
            ),
            ident.span(),
        );
        let argument = Self::argument();
        let parameter = self.parameter.map(|ty| quote!(#argument: #ty));
        let attrs = self.conditional_attrs;
        let value = self.value;
        Ok(quote! {
            #(#attrs)*
            #[doc = #documentation]
            #[allow(dead_code, non_snake_case)]
            #visibility fn #ident(#parameter) -> #root {
                #value
            }
        })
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the configuration attribute of nested algebraic declarations.

    use super::*;

    use syn::parse::Parser;

    /// Collects the options of one written configuration attribute.
    fn overrides(source: &str) -> syn::Result<OptionOverrides> {
        let mut attrs = Attribute::parse_outer.parse_str(source)?;
        OptionOverrides::take_from(&mut attrs)
    }

    /// Only caller-written documentation appears on declarations and members.
    #[test]
    fn preserves_docs_without_placeholder_descriptions() {
        let output = strutuct(quote! {
            /// Root description.
            pub Root {
                /// Child field description.
                pub child: Child { pub value: u8 },
                pub kind: Kind {
                    /// Known variant description.
                    Known,
                    Unknown,
                },
            }
        });
        assert!(!output.to_string().contains("generated by"));
        let file: syn::File = parse2(output).unwrap();
        let docs = |attrs: &[Attribute]| {
            attrs
                .iter()
                .filter(|attr| attr.path().is_ident("doc"))
                .count()
        };
        for item in file.items {
            match item {
                syn::Item::Struct(item) if item.ident == "Root" => {
                    assert_eq!(docs(&item.attrs), 1);
                    assert_eq!(docs(&item.fields.iter().next().unwrap().attrs), 1);
                    assert_eq!(docs(&item.fields.iter().nth(1).unwrap().attrs), 0);
                }
                syn::Item::Struct(item) if item.ident == "RootChild" => {
                    assert_eq!(docs(&item.attrs), 0);
                    assert_eq!(docs(&item.fields.iter().next().unwrap().attrs), 0);
                }
                syn::Item::Enum(item) if item.ident == "RootKind" => {
                    assert_eq!(docs(&item.attrs), 0);
                    assert_eq!(docs(&item.variants[0].attrs), 1);
                    assert_eq!(docs(&item.variants[1].attrs), 0);
                }
                _ => {}
            }
        }
    }

    /// A visibility entry coexists with named flags and a trailing comma.
    #[test]
    fn parses_visibility_beside_named_options() {
        let options = overrides("#[strutuct(pub(crate), reverse_concat,)]").unwrap();

        assert!(matches!(
            options.visibility,
            Some(Visibility::Restricted(_))
        ));
        assert_eq!(options.reverse_concat, Some(true));
        assert_eq!(options.inclusions, None);
    }

    /// `priv` requests the private visibility explicitly.
    #[test]
    fn parses_priv_as_private_visibility() {
        let options = overrides("#[strutuct(priv)]").unwrap();

        assert!(matches!(options.visibility, Some(Visibility::Inherited)));
    }

    /// Declarations and fields are private until an attribute inherits a visibility.
    #[test]
    fn resolves_written_visibility_before_the_inherited_default() {
        let private = Options::default();
        let inherited = private.with(&overrides("#[strutuct(pub)]").unwrap());

        assert!(matches!(
            private.resolve_visibility(None),
            Visibility::Inherited
        ));
        assert!(matches!(
            inherited.resolve_visibility(None),
            Visibility::Public(_)
        ));
        assert!(matches!(
            inherited.resolve_visibility(Some(Visibility::Inherited)),
            Visibility::Inherited
        ));
    }

    /// Configuration attributes are taken while every other attribute stays written.
    #[test]
    fn retains_attributes_other_than_configuration() {
        let mut attrs = Attribute::parse_outer
            .parse_str("#[derive(Debug)] #[strutuct(pub)] #[serde(default)]")
            .unwrap();
        let options = OptionOverrides::take_from(&mut attrs).unwrap();

        assert!(matches!(options.visibility, Some(Visibility::Public(_))));
        assert_eq!(attrs.len(), 2);
        assert!(attrs[0].path().is_ident("derive"));
        assert!(attrs[1].path().is_ident("serde"));
    }

    /// Negated flags disable options enabled by an enclosing declaration.
    #[test]
    fn negated_flags_override_inherited_options() {
        let enabled = Options::default()
            .with(&overrides("#[strutuct(inclusions, product_variants, reverse_concat)]").unwrap());
        assert!(enabled.inclusions);
        assert!(enabled.product_variants);
        assert!(matches!(enabled.concat_order, ConcatOrder::Reversed));

        let disabled = enabled.with(
            &overrides("#[strutuct(no_inclusions, no_product_variants, no_reverse_concat)]")
                .unwrap(),
        );
        assert!(!disabled.inclusions);
        assert!(!disabled.product_variants);
        assert!(matches!(disabled.concat_order, ConcatOrder::Written));
    }

    /// Unknown, repeated and boolean-valued entries are rejected with specific messages.
    #[test]
    fn rejects_malformed_options() {
        let message = |source: &str| {
            overrides(source)
                .err()
                .map(|error| error.to_string())
                .unwrap_or_default()
        };

        assert!(message("#[strutuct(public = true)]").contains("unknown `strutuct` option"));
        assert!(message("#[strutuct(pub, priv)]").contains("duplicate `strutuct` visibility"));
        assert!(
            message("#[strutuct(inclusions, no_inclusions)]")
                .contains("duplicate `strutuct` option")
        );
        for option in ["inclusions", "product_variants", "reverse_concat"] {
            for value in ["true", "false"] {
                assert!(
                    message(&format!("#[strutuct({option} = {value})]"))
                        .contains("use the bare flag")
                );
                assert!(
                    message(&format!("#[strutuct(no_{option} = {value})]"))
                        .contains("use the bare flag")
                );
            }
            for flags in [
                format!("{option}, no_{option}"),
                format!("no_{option}, {option}"),
            ] {
                assert!(
                    message(&format!("#[strutuct({flags})]"))
                        .contains("duplicate `strutuct` option")
                );
            }
        }
    }
}
