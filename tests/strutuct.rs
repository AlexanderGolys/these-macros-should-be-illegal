//! Consumer tests for nested algebraic declarations and constructor macros.

use serde::Deserialize;
use these_macros_should_be_illegal::{emmun, strutuct};

/// Leaf payload used by the nested-struct consumer test.
pub struct B(pub u8);

/// Generic wrapper used to prove that generated declarations can be nested in type arguments.
#[derive(Debug, PartialEq)]
pub struct Delimited<T>(pub T);

strutuct! {
    S
    a: A { A1, A2, A3 },
    b: B,
}

/// Prefix-expression payload used to exercise transitive construction.
pub struct Pref(pub u8);
/// Postfix-expression payload used to exercise transitive construction.
pub struct Post(pub u8);
/// Binary-expression payload used to exercise direct construction.
pub struct Bin(pub u8);

strutuct! {
    Expr
    Unary { (Pref), (Post) }
    (Bin)
    LitStr(String)
    Null
}

strutuct! {
    Message
    Payload { value: String }
    Empty
}

strutuct! {
    Wrapped
    optional: Pref?,
    boxed: Post*,
}

strutuct! {
    Product
    Pair(String, u8)
}

strutuct! {
    Outer
    Middle { Inner { Leaf { Done(String) } } }
}

strutuct! {
    TypedChoices
    Named |NamedPayload| { A, B }
    Generated { A, B }
    |ImplicitPayload| { A, B }
    NamedUnit |NamedUnitPayload|
    |ImplicitUnit|
}

strutuct! {
    UnitComponent
    generated: |GeneratedUnit|,
}

strutuct! {
    RootTuple
    (String, u8)
}

strutuct! {
    TuplePayloads
    Auto { (String, u8) }
    Explicit |ExplicitPair| { (String, u8) }
    |ImplicitPair| { (String, u8) }
}

strutuct! {
    #[strutuct(product_variants = false)]
    RustVariants
    Pair(String, u8)
    Struct { x: String, y: u8 }
    #[strutuct(product_variants = true)]
    Packed(String, u8)
    #[strutuct(product_variants = true)]
    Generated { x: String, y: u8 }
}

strutuct! {
    #[derive(Debug, Deserialize, PartialEq)]
    SerdeSettings
    #[serde(flatten)]
    theme: SerdeTheme {
        #[serde(default)]
        label: Option<String>,
        palette: SerdePalette { Light, Dark },
    },
}

strutuct! {
    #[derive(Debug, Default, PartialEq)]
    DefaultFamily
    settings: DefaultSettings {
        choice: DefaultChoice {
            #[default]
            First,
            Second,
        },
    },
}

strutuct! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    DeriveFamily
    #[derive(Copy, Hash)]
    kind: LiteralKind {
        Nested { value: u8 },
        Unit,
    },
}

strutuct! {
    #[derive(Debug, PartialEq)]
    GenericContainer
    #[derive(Clone, Eq)]
    arguments: Delimited<Option<ArgumentListContent {
        Empty,
        Values(String),
    }>>,
}

emmun! {
    #[derive(Debug, PartialEq)]
    pub struct Aliased {
        value: u8,
    }
}

strutuct! {
    #[derive(Debug, PartialEq)]
    #[strutuct(reverse_concat = true)]
    pub enum Reversed {
        Branch { Leaf { End(String) } }
    }
}

strutuct! {
    #[derive(Debug, PartialEq)]
    #[strutuct(public = false)]
    struct PrivateDefaults {
        value: u8,
        #[strutuct(public = true)]
        #[strutuct(reverse_concat = true)]
        pub branch: pub enum BranchChoice {
            Nested { Unit }
        },
        #[strutuct(product_variants = false)]
        pair: enum LocalProduct { Pair(u8, u8) },
    }
}

strutuct! {
    #[derive(Debug, PartialEq)]
    pub struct GenericVec {
        choices: Vec<enum InlineChoice { First, Second }>,
    }
}

/// Verifies that nested declarations are emitted before their parent struct.
#[test]
fn hoists_nested_declarations_into_ordinary_public_types() {
    let value = S!(a: A!(A2), b: B(7));

    assert!(matches!(value.a, A::A2));
    assert_eq!(value.b.0, 7);
}

/// Verifies direct construction from leaves through multiple generated enums.
#[test]
fn composes_automatic_constructors_through_nested_enums() {
    let prefix = Expr!(Unary::Pref(1));
    let postfix = Expr!(Unary::Post(2));
    let binary = Expr!(Bin(3));

    assert!(matches!(
        prefix,
        Expr::Unary(ExprUnary::PrefExprUnary(Pref(1)))
    ));
    assert!(matches!(
        postfix,
        Expr::Unary(ExprUnary::PostExprUnary(Post(2)))
    ));
    assert!(matches!(binary, Expr::BinExpr(Bin(3))));
}

/// Verifies that explicitly named Rust-like variants retain their constructors.
#[test]
fn leaves_explicit_enum_constructors_ordinary() {
    let literal = Expr!(LitStr("text".to_owned()));
    let null = Expr!(Null);

    assert!(matches!(literal, Expr::LitStr(value) if value == "text"));
    assert!(matches!(null, Expr::Null));
}

/// Packs a multi-field variant into one tuple product.
#[test]
fn packs_tuple_like_variant_products() {
    let pair = Product!(Pair("left".to_owned(), 7));

    assert!(matches!(pair, Product::Pair((value, 7)) if value == "left"));
}

/// Verifies nested structs as variants and both postfix type transformations.
#[test]
fn constructs_nested_struct_variants_and_postfix_types() {
    let message = Message!(Payload {
        value: "payload".to_owned()
    });
    let wrapped = Wrapped!(
        optional: Some(Pref(4)),
        boxed: Box::new(Post(5)),
    );

    assert!(matches!(
        message,
        Message::Payload(MessagePayload { value }) if value == "payload"
    ));
    assert!(matches!(wrapped.optional, Some(Pref(4))));
    assert_eq!(wrapped.boxed.0, 5);
}

/// Verifies that every generated enum macro consumes exactly one path segment.
#[test]
fn folds_a_complete_nested_enum_path() {
    let value = Outer!(Middle::Inner::Leaf::Done("finished".to_owned()));

    assert!(matches!(
        value,
        Outer::Middle(OuterMiddle::Inner(OuterMiddleInner::Leaf(
            OuterMiddleInnerLeaf::Done(message)
        )))
            if message == "finished"
    ));
}

/// Distinguishes explicit type names, generated type names, and implicit type choices.
#[test]
fn names_generated_types_from_parents_and_variants() {
    let named = TypedChoices!(Named::A);
    let generated = TypedChoices!(Generated::B);
    let implicit = TypedChoices!(ImplicitPayload::A);

    assert!(matches!(named, TypedChoices::Named(NamedPayload::A)));
    assert!(matches!(
        generated,
        TypedChoices::Generated(TypedChoicesGenerated::B)
    ));
    assert!(matches!(
        implicit,
        TypedChoices::ImplicitPayloadTypedChoices(ImplicitPayload::A)
    ));
}

/// Generates nominal unit types when a bar-named declaration omits its body.
#[test]
fn generates_named_unit_types_without_bodies() {
    let named = TypedChoices!(NamedUnit);
    let implicit = TypedChoices!(ImplicitUnit);
    let component = UnitComponent!(generated: GeneratedUnit);

    assert!(matches!(named, TypedChoices::NamedUnit(NamedUnitPayload)));
    assert!(matches!(
        implicit,
        TypedChoices::ImplicitUnitTypedChoices(ImplicitUnit)
    ));
    assert!(matches!(component.generated, GeneratedUnit));
}

/// Generates root and nested tuple products with at least two elements.
#[test]
fn generates_tuple_declarations() {
    let root = RootTuple!("root".to_owned(), 1);
    let automatic = TuplePayloads!(Auto("auto".to_owned(), 2));
    let explicit = TuplePayloads!(Explicit("explicit".to_owned(), 3));
    let implicit = TuplePayloads!(ImplicitPair("implicit".to_owned(), 4));

    assert!(matches!(root, RootTuple(value, 1) if value == "root"));
    assert!(matches!(
        automatic,
        TuplePayloads::Auto(TuplePayloadsAuto(value, 2)) if value == "auto"
    ));
    assert!(matches!(
        explicit,
        TuplePayloads::Explicit(ExplicitPair(value, 3)) if value == "explicit"
    ));
    assert!(matches!(
        implicit,
        TuplePayloads::ImplicitPairTuplePayloads(ImplicitPair(value, 4))
            if value == "implicit"
    ));
}

/// Applies a declaration-wide product option and permits rare variant overrides.
#[test]
fn configures_product_variants_with_outer_attributes() {
    let pair = RustVariants!(Pair("pair".to_owned(), 1));
    let structure = RustVariants!(Struct {
        x: "struct".to_owned(),
        y: 2,
    });
    let packed = RustVariants!(Packed("packed".to_owned(), 3));
    let generated = RustVariants!(Generated {
        x: "generated".to_owned(),
        y: 4,
    });

    assert!(matches!(
        pair,
        RustVariants::Pair(value, 1) if value == "pair"
    ));
    assert!(matches!(
        structure,
        RustVariants::Struct { x: value, y: 2 } if value == "struct"
    ));
    assert!(matches!(
        packed,
        RustVariants::Packed((value, 3)) if value == "packed"
    ));
    assert!(matches!(
        generated,
        RustVariants::Generated(RustVariantsGenerated { x: value, y: 4 })
            if value == "generated"
    ));
}

/// Propagates derives to nested types and preserves serde field attributes.
#[test]
fn supports_serde_attributes_across_nested_declarations() {
    fn assert_deserialize<T>()
    where
        T: for<'de> Deserialize<'de>,
    {
    }

    assert_deserialize::<SerdeSettings>();
    assert_deserialize::<SerdeTheme>();
    assert_deserialize::<SerdePalette>();
    assert_eq!(
        SerdeTheme {
            label: None,
            palette: SerdePalette::Light,
        },
        SerdeTheme {
            label: None,
            palette: SerdePalette::Light,
        }
    );
}

/// Uses Rust's ordinary default-variant attribute inside a derived family.
#[test]
fn derives_defaults_for_nested_structs_and_enums() {
    assert_eq!(
        DefaultFamily::default().settings.choice,
        DefaultChoice::First
    );
}

/// Adds local derives without removing traits inherited from the type family.
#[test]
fn combines_inherited_and_local_derives() {
    fn assert_traits<T>()
    where
        T: std::fmt::Debug + Clone + PartialEq + Eq + Copy + std::hash::Hash,
    {
    }

    assert_traits::<LiteralKind>();
    assert_traits::<LiteralKindNested>();
}

/// Hoists declarations nested inside arbitrarily deep ordinary generic arguments.
#[test]
fn generates_types_inside_generic_containers() {
    fn assert_nested_traits<T>()
    where
        T: std::fmt::Debug + PartialEq + Clone + Eq,
    {
    }

    assert_nested_traits::<ArgumentListContent>();

    let value = GenericContainer!(
        arguments: Delimited(Some(ArgumentListContent!(Values(
            "argument".to_owned()
        )))),
    );

    assert_eq!(
        value,
        GenericContainer {
            arguments: Delimited(Some(ArgumentListContent::Values("argument".to_owned()))),
        }
    );
}

/// Exposes `emmun!` as an exact alias for the complete `strutuct!` expansion.
#[test]
fn expands_the_emmun_alias() {
    assert_eq!(Aliased!(value: 7), Aliased { value: 7 });
}

/// Reverses every automatically concatenated name in one declaration family.
#[test]
fn reverses_generated_name_concatenation() {
    let value = Reversed!(Branch::Leaf::End("done".to_owned()));

    assert!(matches!(
        value,
        Reversed::Branch(BranchReversed::Leaf(LeafBranchReversed::End(message)))
            if message == "done"
    ));
}

/// Lets global private defaults and local public overrides coexist in one family.
#[test]
fn configures_visibility_globally_and_locally() {
    let value = PrivateDefaults!(
        value: 9,
        branch: BranchChoice!(Nested::Unit),
        pair: LocalProduct!(Pair(1, 2)),
    );

    assert_eq!(value.value, 9);
    assert!(matches!(
        value.branch,
        BranchChoice::Nested(NestedBranchChoice::Unit)
    ));
    assert!(matches!(value.pair, LocalProduct::Pair(1, 2)));
}

/// Generates a declaration directly inside a standard generic container.
#[test]
fn generates_an_enum_inside_vec() {
    let value = GenericVec!(choices: vec![InlineChoice!(Second)]);

    assert!(matches!(value.choices.as_slice(), [InlineChoice::Second]));
}
