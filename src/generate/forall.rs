//! One shared declaration prefix repeated on every field or variant of a data item.

use proc_macro2::{Delimiter, TokenStream};
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Error, Field, Fields, FieldsNamed, FieldsUnnamed, Ident, LitStr,
    Token, Type, Variant, Visibility,
    parse::{Parse, ParseStream},
    parse_quote, parse2,
    punctuated::Punctuated,
    token::Brace,
};

use crate::helpers::ungroup;

/// Repeats one shared member declaration prefix over every field or variant.
pub fn forall(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let result = parse2::<MemberPrefix>(arguments)
        .and_then(|prefix| parse2::<DeriveInput>(item).map(|item| (prefix, item)))
        .and_then(|(prefix, mut item)| prefix.distribute(&mut item).map(|()| quote!(#item)));

    result.unwrap_or_else(Error::into_compile_error)
}

/// The constant leading part of a member declaration: `#[attribute] pub Type`.
struct MemberPrefix {
    /// Outer attributes added in front of the ones each member carries.
    attributes: Vec<Attribute>,
    /// Visibility of the members that declare none themselves.
    visibility: Option<Visibility>,
    /// Fields every member is given beside the ones it declares.
    shared: SharedFields,
}

/// The fields the prefix gives to every member of the annotated item.
enum SharedFields {
    /// Nothing is added, and every member keeps the fields it declares.
    Declared,
    /// One unnamed field per shared type.
    Unnamed(Vec<Type>),
    /// One group of named fields, written as `{ name: Type, ... }`.
    Named(FieldsNamed),
}

/// Rejection of a prefix naming both the shared types and a group of named fields.
const MIXED_SHARED_FIELDS: &str =
    "`forall` distributes either shared types or one group of named fields";

impl Parse for MemberPrefix {
    /// Parses the parts of the prefix in any order, with optional commas between them.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut prefix = Self {
            attributes: Vec::new(),
            visibility: None,
            shared: SharedFields::Declared,
        };
        while !input.is_empty() {
            prefix.parse_part(input)?;
            input.parse::<Option<Token![,]>>()?;
        }
        if prefix.attributes.is_empty()
            && prefix.visibility.is_none()
            && matches!(prefix.shared, SharedFields::Declared)
        {
            return Err(input.error(
                "expected an attribute, a visibility, a type or a group of named fields to distribute",
            ));
        }

        Ok(prefix)
    }
}

/// Recognition of each part of the prefix by the syntax that only it has.
impl MemberPrefix {
    /// Reads one part of the prefix, rejecting the repetition of a unique one.
    fn parse_part(&mut self, input: ParseStream) -> syn::Result<()> {
        if skipped_empty_fragment(input) {
            return Ok(());
        }

        if input.peek(Token![#]) {
            if input.peek2(Token![!]) {
                return Err(input.error(
                    "`forall` distributes outer attributes, since an inner one cannot apply to a field or variant",
                ));
            }
            self.attributes.extend(input.call(Attribute::parse_outer)?);
            return Ok(());
        }

        if input.peek(Token![pub]) {
            if self.visibility.is_some() {
                return Err(input.error("`forall` distributes one visibility"));
            }
            self.visibility = Some(input.parse()?);
            return Ok(());
        }

        if input.peek(Brace) {
            let group = input.parse::<FieldsNamed>()?;
            if group.named.is_empty() {
                return Err(Error::new_spanned(
                    group,
                    "expected at least one named field in the group",
                ));
            }
            match &self.shared {
                SharedFields::Named(_) => {
                    return Err(Error::new_spanned(
                        group,
                        "`forall` distributes one group of named fields",
                    ));
                }
                SharedFields::Unnamed(_) => {
                    return Err(Error::new_spanned(group, MIXED_SHARED_FIELDS));
                }
                SharedFields::Declared => self.shared = SharedFields::Named(group),
            }
            return Ok(());
        }

        let shared = input.parse::<Type>()?;
        if let SharedFields::Named(_) = &self.shared {
            return Err(Error::new_spanned(shared, MIXED_SHARED_FIELDS));
        }
        if let SharedFields::Unnamed(types) = &mut self.shared {
            types.push(shared);
            return Ok(());
        }
        self.shared = SharedFields::Unnamed(vec![shared]);

        Ok(())
    }
}

/// Distribution of the prefix over the members of each kind of data item.
impl MemberPrefix {
    /// Rewrites every member of one struct, union or enum with the shared prefix.
    fn distribute(&self, item: &mut DeriveInput) -> syn::Result<()> {
        let owner = &item.ident;
        match &mut item.data {
            Data::Struct(data) => {
                if data.fields.is_empty() {
                    return Err(nothing_to_distribute(owner, "fields"));
                }
                if let SharedFields::Named(group) = &self.shared {
                    return Err(variant_fields_only(group));
                }
                match self.shared_type(owner)? {
                    None => data.fields.iter_mut().for_each(|field| self.apply(field)),
                    Some(shared) => {
                        let Fields::Unnamed(names) = &data.fields else {
                            return Err(fields_already_typed(shared, owner));
                        };
                        let named = self.name_fields(names, shared, owner)?;
                        data.fields = Fields::Named(named);
                    }
                }
            }
            Data::Union(data) => {
                if data.fields.named.is_empty() {
                    return Err(nothing_to_distribute(owner, "fields"));
                }
                if let SharedFields::Named(group) = &self.shared {
                    return Err(variant_fields_only(group));
                }
                if let Some(shared) = self.shared_type(owner)? {
                    return Err(fields_already_typed(shared, owner));
                }
                data.fields
                    .named
                    .iter_mut()
                    .for_each(|field| self.apply(field));
            }
            Data::Enum(data) => {
                if data.variants.is_empty() {
                    return Err(nothing_to_distribute(owner, "variants"));
                }
                if let Some(visibility) = &self.visibility {
                    return Err(Error::new_spanned(
                        visibility,
                        "an enum variant cannot have a visibility",
                    ));
                }
                for variant in &mut data.variants {
                    self.extend(variant, owner)?;
                }
            }
        }

        Ok(())
    }

    /// Reads the one type every field of a struct or union is given.
    fn shared_type(&self, owner: &Ident) -> syn::Result<Option<&Type>> {
        let SharedFields::Unnamed(types) = &self.shared else {
            return Ok(None);
        };
        match types.as_slice() {
            [shared] => Ok(Some(shared)),
            [_, surplus, ..] => Err(Error::new_spanned(
                surplus,
                format!(
                    "a field of `{owner}` holds one type, but `forall` received {}",
                    types.len()
                ),
            )),
            [] => Ok(None),
        }
    }

    /// Adds the shared attributes to one field and fills in its implicit visibility.
    fn apply(&self, field: &mut Field) {
        field.attrs = self.inherited_by(std::mem::take(&mut field.attrs));
        if matches!(field.vis, Visibility::Inherited)
            && let Some(visibility) = &self.visibility
        {
            field.vis = visibility.clone();
        }
    }

    /// Adds the shared attributes to one variant and the shared fields to its own.
    fn extend(&self, variant: &mut Variant, owner: &Ident) -> syn::Result<()> {
        variant.attrs = self.inherited_by(std::mem::take(&mut variant.attrs));
        match &self.shared {
            SharedFields::Declared => Ok(()),
            SharedFields::Unnamed(types) => {
                reject_discriminant(variant)?;
                append_types(variant, types)
            }
            SharedFields::Named(group) => {
                reject_discriminant(variant)?;
                add_named_fields(variant, group, owner)
            }
        }
    }

    /// Puts the shared attributes in front of the ones a member declares itself.
    fn inherited_by(&self, own: Vec<Attribute>) -> Vec<Attribute> {
        self.attributes.iter().cloned().chain(own).collect()
    }

    /// Rewrites bare names written in tuple position as named fields of one shared type.
    fn name_fields(
        &self,
        names: &FieldsUnnamed,
        shared: &Type,
        owner: &Ident,
    ) -> syn::Result<FieldsNamed> {
        let mut named = Punctuated::<Field, Token![,]>::new();
        for position in &names.unnamed {
            let name = field_name(position)?;
            let mut field: Field = parse_quote!(#name: #shared);
            field.attrs = position.attrs.clone();
            field.vis = position.vis.clone();
            self.apply(&mut field);
            document_created_field(&mut field, &owner.to_string(), &name);
            named.push(field);
        }

        Ok(parse_quote!({ #named }))
    }
}

/// Appends one unnamed field per shared type to the fields of one variant.
fn append_types(variant: &mut Variant, types: &[Type]) -> syn::Result<()> {
    match &mut variant.fields {
        Fields::Unnamed(fields) => {
            fields.unnamed.extend(
                types
                    .iter()
                    .map(|shared| -> Field { parse_quote!(#shared) }),
            );
            Ok(())
        }
        Fields::Named(fields) => Err(Error::new_spanned(
            fields,
            format!(
                "variant `{}` names its fields, so `forall` cannot append the shared types",
                variant.ident
            ),
        )),
        Fields::Unit => {
            variant.fields = Fields::Unnamed(parse_quote!((#(#types),*)));
            Ok(())
        }
    }
}

/// Adds the shared named fields to one variant, keeping the ones it declares.
fn add_named_fields(variant: &mut Variant, group: &FieldsNamed, owner: &Ident) -> syn::Result<()> {
    let scope = format!("{owner}::{}", variant.ident);
    let mut shared = group.named.clone();
    for field in &mut shared {
        let name = field
            .ident
            .clone()
            .ok_or_else(|| Error::new_spanned(&*field, "expected a named field"))?;
        document_created_field(field, &scope, &name);
    }

    match &mut variant.fields {
        Fields::Named(fields) => {
            if let Some(declared) = fields
                .named
                .iter()
                .find(|declared| shared.iter().any(|field| declared.ident == field.ident))
            {
                return Err(Error::new_spanned(
                    declared,
                    format!(
                        "variant `{}` already declares this field, which `forall` also distributes",
                        variant.ident
                    ),
                ));
            }
            fields.named.extend(shared);
            Ok(())
        }
        Fields::Unnamed(fields) => Err(Error::new_spanned(
            fields,
            format!(
                "variant `{}` holds unnamed fields, so `forall` cannot add the shared named ones",
                variant.ident
            ),
        )),
        Fields::Unit => {
            variant.fields = Fields::Named(parse_quote!({ #shared }));
            Ok(())
        }
    }
}

/// Rejects a variant that cannot hold fields beside its discriminant.
fn reject_discriminant(variant: &Variant) -> syn::Result<()> {
    match &variant.discriminant {
        None => Ok(()),
        Some((_, discriminant)) => Err(Error::new_spanned(
            discriminant,
            format!(
                "variant `{}` has a discriminant, so it cannot hold fields",
                variant.ident
            ),
        )),
    }
}

/// Consumes the empty invisible group a `$:vis` fragment matching nothing leaves behind.
fn skipped_empty_fragment(input: ParseStream) -> bool {
    input
        .step(|cursor| match cursor.group(Delimiter::None) {
            Some((content, _, rest)) if content.eof() => Ok(((), rest)),
            _ => Err(cursor.error("expected a fragment that matched nothing")),
        })
        .is_ok()
}

/// Reads one bare identifier written in tuple-field position as a field name.
fn field_name(position: &Field) -> syn::Result<Ident> {
    match ungroup::ty(&position.ty) {
        Type::Path(path) if path.qself.is_none() => path.path.get_ident().cloned(),
        _ => None,
    }
    .ok_or_else(|| Error::new_spanned(&position.ty, "expected a field name, found a type"))
}

/// Documents one created field where neither the member nor the prefix documents it.
fn document_created_field(field: &mut Field, scope: &str, name: &Ident) {
    if field
        .attrs
        .iter()
        .any(|attribute| attribute.path().is_ident("doc"))
    {
        return;
    }

    let documentation = LitStr::new(
        &format!("Field `{scope}::{name}` generated by `forall`."),
        name.span(),
    );
    field.attrs.push(parse_quote!(#[doc = #documentation]));
}

/// Rejects an item without a single member to repeat the prefix on.
fn nothing_to_distribute(owner: &Ident, members: &str) -> Error {
    Error::new_spanned(
        owner,
        format!("`forall` has nothing to distribute: `{owner}` has no {members}"),
    )
}

/// Rejects a shared type for members that already name the type they hold.
fn fields_already_typed(shared: &Type, owner: &Ident) -> Error {
    Error::new_spanned(
        shared,
        format!("the fields of `{owner}` already name the type they hold"),
    )
}

/// Rejects a group of named fields for members other than enum variants.
fn variant_fields_only(group: &FieldsNamed) -> Error {
    Error::new_spanned(
        group,
        "`forall` adds a group of named fields to the variants of an enum",
    )
}

#[cfg(test)]
mod tests {
    //! Distribution over each member shape, and the diagnostics of the rejected ones.

    use super::*;

    /// Distributes one prefix over one item, both written as source text.
    fn distributed(prefix: &str, item: &str) -> String {
        forall(prefix.parse().unwrap(), item.parse().unwrap()).to_string()
    }

    /// A visibility reaches the types of a tuple struct without renaming them.
    #[test]
    fn distributes_a_visibility_over_tuple_fields() {
        let output = distributed("pub(crate)", "pub struct S(a, b, c);");

        assert_eq!(
            output,
            "pub struct S (pub (crate) a , pub (crate) b , pub (crate) c) ;"
        );
    }

    /// A shared type turns tuple positions into documented named fields.
    #[test]
    fn names_tuple_fields_sharing_one_type() {
        let output = distributed("pub, T", "struct S<T>(a, b) where T: Copy;");

        assert_eq!(
            output,
            concat!(
                "struct S < T > where T : Copy { ",
                r#"# [doc = "Field `S::a` generated by `forall`."] pub a : T , "#,
                r#"# [doc = "Field `S::b` generated by `forall`."] pub b : T "#,
                "}"
            )
        );
    }

    /// The parts of the prefix are recognized wherever they are written.
    #[test]
    fn reads_the_prefix_in_any_order() {
        let output = distributed(r#"u8, #[doc = "shared"], pub(crate)"#, "struct S(a);");

        assert!(
            output.contains(r#"# [doc = "shared"] pub (crate) a : u8"#),
            "{output}"
        );
    }

    /// The shared types reach the members in the order they are written.
    #[test]
    fn keeps_the_order_of_the_shared_types() {
        let output = distributed("Box<T>, T", "enum E<T> { A }");

        assert!(output.contains("A (Box < T > , T)"), "{output}");
    }

    /// A field documented in tuple position keeps its own documentation.
    #[test]
    fn keeps_the_documentation_of_a_named_field() {
        let output = distributed("pub, u8", r#"struct S(#[doc = "own"] a);"#);

        assert!(
            output.contains(r#"# [doc = "own"] pub a : u8 }"#),
            "{output}"
        );
    }

    /// Every variant of an enum receives the shared types as unnamed fields.
    #[test]
    fn appends_shared_types_to_every_variant() {
        let output = distributed("T, Box<T>", "enum E<T> { A, B(i8) }");

        assert!(output.contains("A (T , Box < T >)"), "{output}");
        assert!(output.contains("B (i8 , T , Box < T >)"), "{output}");
    }

    /// A group of named fields reaches every variant of an enum.
    #[test]
    fn adds_shared_named_fields_to_every_variant() {
        let output = distributed("{ code: u16, path: String }", "enum E { A, B { own: u8 } }");

        assert!(
            output.contains(
                r#"A { # [doc = "Field `E::A::code` generated by `forall`."] code : u16"#
            ),
            "{output}"
        );
        assert!(
            output.contains(
                r#"own : u8 , # [doc = "Field `E::B::code` generated by `forall`."] code : u16"#
            ),
            "{output}"
        );
        assert!(output.contains("path : String } }"), "{output}");
    }

    /// Shared attributes precede the attributes a member declares itself.
    #[test]
    fn prepends_shared_attributes() {
        let output = distributed(
            r#"#[doc = "shared"]"#,
            r#"enum E { A {}, #[doc = "own"] B(i8) }"#,
        );

        assert!(output.contains(r#"# [doc = "shared"] A { }"#), "{output}");
        assert!(
            output.contains(r#"# [doc = "shared"] # [doc = "own"] B (i8)"#),
            "{output}"
        );
    }

    /// A member that declares its own visibility keeps it.
    #[test]
    fn keeps_an_explicit_member_visibility() {
        let output = distributed("pub", "struct S { pub(super) hidden: u8, shared: u8 }");

        assert!(output.contains("pub (super) hidden : u8"), "{output}");
        assert!(output.contains("pub shared : u8"), "{output}");
    }

    /// A union distributes over its named fields like a braced struct.
    #[test]
    fn distributes_over_union_fields() {
        let output = distributed(
            r#"#[doc = "raw"] pub"#,
            "union U { integer: u32, float: f32 }",
        );

        assert!(
            output.contains(r#"# [doc = "raw"] pub integer : u32"#),
            "{output}"
        );
        assert!(
            output.contains(r#"# [doc = "raw"] pub float : f32"#),
            "{output}"
        );
    }

    /// Enum variants cannot have a visibility, so distributing one is rejected.
    #[test]
    fn rejects_a_visibility_on_an_enum() {
        let output = distributed("pub", "enum E { A, B }");

        assert!(output.contains("cannot have a visibility"), "{output}");
    }

    /// A struct field holds one type, so several shared types are rejected.
    #[test]
    fn rejects_several_types_on_a_struct() {
        let output = distributed("T, Box<T>", "struct S<T>(a, b);");

        assert!(output.contains("holds one type"), "{output}");
    }

    /// Fields that name their own type cannot be given a shared one.
    #[test]
    fn rejects_a_shared_type_for_named_fields() {
        let output = distributed("T", "struct S<T> { a: u8 }");

        assert!(output.contains("already name the type"), "{output}");
    }

    /// Only the variants of an enum take a group of shared named fields.
    #[test]
    fn rejects_a_field_group_on_a_struct() {
        let tuple = distributed("{ code: u16 }", "struct S(a);");
        let braced = distributed("{ code: u16 }", "struct S { a: u8 }");

        assert!(tuple.contains("variants of an enum"), "{tuple}");
        assert!(braced.contains("variants of an enum"), "{braced}");
    }

    /// One group of shared named fields is the most a prefix distributes.
    #[test]
    fn rejects_a_second_field_group() {
        let output = distributed("{ a: u8 }, { b: u8 }", "enum E { A }");

        assert!(output.contains("one group of named fields"), "{output}");
    }

    /// A member holds either the shared types or the shared named fields.
    #[test]
    fn rejects_types_beside_a_field_group() {
        let group_last = distributed("u8, { code: u16 }", "enum E { A }");
        let group_first = distributed("{ code: u16 }, u8", "enum E { A }");

        assert!(group_last.contains("either shared types"), "{group_last}");
        assert!(group_first.contains("either shared types"), "{group_first}");
    }

    /// A variant holding unnamed fields cannot take the shared named ones.
    #[test]
    fn rejects_a_field_group_for_an_unnamed_variant() {
        let output = distributed("{ code: u16 }", "enum E { A(u8) }");

        assert!(output.contains("holds unnamed fields"), "{output}");
    }

    /// A shared field cannot repeat a name the variant already declares.
    #[test]
    fn rejects_a_field_group_repeating_a_declared_name() {
        let output = distributed("{ code: u16 }", "enum E { A { code: u8 } }");

        assert!(output.contains("already declares this field"), "{output}");
    }

    /// A tuple position standing for a field name must be one bare identifier.
    #[test]
    fn rejects_a_type_in_place_of_a_field_name() {
        let output = distributed("T", "struct S<T>(Box<u8>);");

        assert!(output.contains("expected a field name"), "{output}");
    }

    /// A discriminated variant cannot also hold the shared fields.
    #[test]
    fn rejects_shared_fields_for_a_discriminated_variant() {
        let types = distributed("u8", "enum E { A = 1, B }");
        let group = distributed("{ code: u16 }", "enum E { A = 1, B }");

        assert!(types.contains("has a discriminant"), "{types}");
        assert!(group.contains("has a discriminant"), "{group}");
    }

    /// An item without members has nothing to distribute over.
    #[test]
    fn rejects_an_item_without_members() {
        let unit = distributed("pub", "struct S;");
        let empty = distributed("pub", "enum E {}");

        assert!(unit.contains("`S` has no fields"), "{unit}");
        assert!(empty.contains("`E` has no variants"), "{empty}");
    }

    /// An empty argument list names nothing to repeat.
    #[test]
    fn rejects_an_empty_prefix() {
        let output = distributed("", "struct S(u8);");

        assert!(output.contains("expected an attribute"), "{output}");
    }

    /// One visibility is the most a prefix distributes.
    #[test]
    fn rejects_a_second_visibility() {
        let output = distributed("pub, pub(crate)", "struct S(a);");

        assert!(output.contains("one visibility"), "{output}");
    }

    /// Only a struct, an enum and a union declare members.
    #[test]
    fn rejects_an_item_without_fields_or_variants() {
        let output = distributed("pub", "fn standalone() {}");

        assert!(output.contains("expected one of"), "{output}");
    }

    /// A field or variant carries no inner attribute, so the prefix holds none.
    #[test]
    fn rejects_an_inner_attribute() {
        let output = distributed(r#"#![doc = "x"], pub"#, "struct S(a);");

        assert!(output.contains("outer attributes"), "{output}");
    }

    /// A group distributing no field at all is rejected rather than emptying a variant.
    #[test]
    fn rejects_an_empty_field_group() {
        let output = distributed("{}", "enum E { A }");

        assert!(output.contains("at least one named field"), "{output}");
    }

    /// Every restricted form of visibility reaches the fields unchanged.
    #[test]
    fn distributes_a_restricted_visibility() {
        let scoped = distributed("pub(in crate::inner), u8", "struct S(a);");
        let own = distributed("pub(self), u8", "struct S(a);");
        let parent = distributed("pub(super), u8", "struct S(a);");

        assert!(
            scoped.contains("pub (in crate :: inner) a : u8"),
            "{scoped}"
        );
        assert!(own.contains("pub (self) a : u8"), "{own}");
        assert!(parent.contains("pub (super) a : u8"), "{parent}");
    }

    /// A raw identifier in tuple position names a field whose name is a keyword.
    #[test]
    fn names_raw_identifier_fields() {
        let output = distributed("pub, u8", "struct S(r#type, r#fn);");

        assert!(output.contains("pub r#type : u8"), "{output}");
        assert!(output.contains("pub r#fn : u8"), "{output}");
    }

    /// Generics, their bounds and their defaults are kept as the item declares them.
    #[test]
    fn keeps_generics_bounds_and_lifetimes() {
        let output = distributed(
            "pub, &'a [T; N]",
            "struct S<'a, T: Copy = u8, const N: usize>(first, second) where T: Default;",
        );

        assert!(
            output
                .contains("struct S < 'a , T : Copy = u8 , const N : usize > where T : Default {"),
            "{output}"
        );
        assert!(output.contains("pub first : & 'a [T ; N]"), "{output}");
    }

    /// Bounds of an enum survive the variants gaining the shared fields.
    #[test]
    fn keeps_the_bounds_of_an_enum() {
        let output = distributed(
            "{ value: T }",
            "enum E<T: Copy> where T: Default { A, B { own: u8 } }",
        );

        assert!(
            output.contains("enum E < T : Copy > where T : Default"),
            "{output}"
        );
        assert!(output.contains("A { # [doc"), "{output}");
    }

    /// A shared type may be any type, including the exotic spellings of one.
    #[test]
    fn shares_exotic_types() {
        let object = distributed("pub, Box<dyn Fn(u8) -> u8 + Send>", "struct S(call);");
        let ranked = distributed("pub, for<'a> fn(&'a u8) -> &'a u8", "struct S(call);");
        let qualified = distributed("pub, <T as IntoIterator>::Item", "struct S<T>(item);");
        let recursive = distributed("pub, Option<Box<Self>>", "struct S(next);");
        let generated = distributed("pub, qf!(Vec<u8>)", "struct S(values);");

        assert!(
            object.contains("pub call : Box < dyn Fn (u8) -> u8 + Send >"),
            "{object}"
        );
        assert!(
            ranked.contains("pub call : for < 'a > fn (& 'a u8) -> & 'a u8"),
            "{ranked}"
        );
        assert!(
            qualified.contains("pub item : < T as IntoIterator > :: Item"),
            "{qualified}"
        );
        assert!(
            recursive.contains("pub next : Option < Box < Self > >"),
            "{recursive}"
        );
        assert!(
            generated.contains("pub values : qf ! (Vec < u8 >)"),
            "{generated}"
        );
    }

    /// A tuple position holding anything but a bare name is rejected once named.
    #[test]
    fn rejects_compound_types_in_place_of_a_field_name() {
        for position in ["<T as Trait>::Out", "(u8, u8)", "&'a str", "[u8; 2]"] {
            let output = distributed("u8", &format!("struct S<'a, T>({position});"));

            assert!(output.contains("expected a field name"), "{output}");
        }
    }

    /// Conditional compilation reaches every member like any other attribute.
    #[test]
    fn distributes_a_conditional_attribute() {
        let output = distributed(
            r#"#[cfg(all())] #[cfg_attr(all(), doc = "d")] pub"#,
            "struct S(a);",
        );

        assert!(output.contains("# [cfg (all ())]"), "{output}");
        assert!(
            output.contains(r#"# [cfg_attr (all () , doc = "d")]"#),
            "{output}"
        );
    }

    /// Attributes shared by the prefix document a created field, so none is generated.
    #[test]
    fn counts_a_shared_document_as_the_documentation_of_a_field() {
        let output = distributed(r#"#[doc = "shared"] pub, u8"#, "struct S(a);");

        assert!(
            output.contains(r#"# [doc = "shared"] pub a : u8"#),
            "{output}"
        );
        assert!(!output.contains("generated by `forall`"), "{output}");
    }

    /// A variant keeps the discriminant that only the shared fields would conflict with.
    #[test]
    fn keeps_a_discriminant_beside_the_shared_attributes() {
        let output = distributed(r#"#[doc = "d"]"#, "enum E { A = 1, B = 2 }");

        assert!(output.contains(r#"# [doc = "d"] A = 1"#), "{output}");
        assert!(output.contains(r#"# [doc = "d"] B = 2"#), "{output}");
    }

    /// A lifetime stands for no part of a member declaration.
    #[test]
    fn rejects_a_lifetime_as_a_shared_type() {
        let output = distributed("'a", "struct S(a);");

        assert!(output.contains("compile_error"), "{output}");
    }

    /// The shared named fields may borrow the lifetimes of the enum.
    #[test]
    fn shares_named_fields_borrowing_the_item() {
        let output = distributed("{ name: &'a str, }", "enum E<'a> { A }");

        assert!(output.contains("A { # [doc"), "{output}");
        assert!(output.contains("name : & 'a str"), "{output}");
    }
}
