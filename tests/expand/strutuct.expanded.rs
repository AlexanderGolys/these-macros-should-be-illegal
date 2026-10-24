#![no_main]
use these_macros_should_be_illegal::strutuct;
pub struct Existing;
pub struct Delimited<T>(pub T);
pub struct Pref(pub u8);
pub struct Post(pub u8);
pub struct Bin(pub u8);
mod nested_enum_in_struct_field {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    enum SA {
        A1,
        A2,
        A3,
    }
    #[allow(dead_code)]
    struct S {
        a: SA,
        b: Existing,
    }
}
mod local_derive_extends_the_nested_branch {
    use super::*;
    #[allow(dead_code)]
    struct RootBranchLeaf {
        value: String,
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::clone::Clone for RootBranchLeaf {
        #[inline]
        fn clone(&self) -> RootBranchLeaf {
            RootBranchLeaf {
                value: ::core::clone::Clone::clone(&self.value),
            }
        }
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::default::Default for RootBranchLeaf {
        #[inline]
        fn default() -> RootBranchLeaf {
            RootBranchLeaf {
                value: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::fmt::Debug for RootBranchLeaf {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "RootBranchLeaf",
                "value",
                &&self.value,
            )
        }
    }
    #[allow(dead_code)]
    struct RootBranch {
        leaf: RootBranchLeaf,
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::clone::Clone for RootBranch {
        #[inline]
        fn clone(&self) -> RootBranch {
            RootBranch {
                leaf: ::core::clone::Clone::clone(&self.leaf),
            }
        }
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::default::Default for RootBranch {
        #[inline]
        fn default() -> RootBranch {
            RootBranch {
                leaf: ::core::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::fmt::Debug for RootBranch {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "RootBranch",
                "leaf",
                &&self.leaf,
            )
        }
    }
    #[allow(dead_code)]
    struct Root {
        branch: RootBranch,
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::clone::Clone for Root {
        #[inline]
        fn clone(&self) -> Root {
            Root {
                branch: ::core::clone::Clone::clone(&self.branch),
            }
        }
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::default::Default for Root {
        #[inline]
        fn default() -> Root {
            Root {
                branch: ::core::default::Default::default(),
            }
        }
    }
}
mod rust_types_stay_verbatim {
    use super::*;
    #[allow(dead_code)]
    struct Root {
        map: std::collections::HashMap<fn() -> u8, Box<dyn Fn(u8) -> u8>>,
        pointer: fn(u8) -> u8,
        array: [u8; 2],
        next: u8,
    }
}
mod relative_names_across_type_contexts {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    enum RootDirect {
        A,
        B,
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum RootOptional {
        A,
        B,
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum RootBoxed {
        A,
        B,
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum RootTupleElement {
        A,
        B,
    }
    #[allow(dead_code)]
    struct RootTuple(u8, RootTupleElement);
    #[allow(dead_code, clippy::enum_variant_names)]
    enum ExactChild {
        A,
        B,
    }
    #[allow(dead_code)]
    struct Exact {
        child: ExactChild,
    }
    #[allow(dead_code)]
    struct Root {
        direct: RootDirect,
        optional: ::core::option::Option<RootOptional>,
        boxed: ::std::boxed::Box<RootBoxed>,
        tuple: RootTuple,
        exact: Exact,
        existing: Existing,
    }
}
mod postfix_type_operators {
    use super::*;
    #[allow(dead_code)]
    struct S {
        optional: ::core::option::Option<Existing>,
        nested: ::std::boxed::Box<::core::option::Option<Existing>>,
    }
}
mod type_macro_inputs_stay_opaque {
    use super::*;
    #[allow(dead_code)]
    struct Root {
        value: Delimited<u8>,
    }
}
mod braced_keyword_declarations_and_visibility {
    use super::*;
    #[allow(dead_code)]
    pub(crate) struct Product {
        hidden: u8,
        pub shown: u8,
    }
    #[allow(dead_code)]
    struct Private {
        field: u8,
    }
}
mod keywords_on_exact_payloads {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    enum Choice {
        A,
        B,
    }
    #[allow(dead_code)]
    struct Marker;
    #[allow(dead_code, clippy::enum_variant_names)]
    enum Root {
        ChoiceRoot(Choice),
        Named(Marker),
    }
}
mod rust_variant_shapes {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    enum RootKind {
        A,
        B,
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum RootNested {
        Leaf,
    }
    #[allow(dead_code)]
    struct RootTuple(u8, u8);
    #[allow(dead_code, clippy::enum_variant_names)]
    enum Root {
        Unit,
        EmptyTuple(),
        Single(u8),
        Pair(u8, Existing),
        EmptyRecord {},
        Record { value: u8, kind: RootKind },
        Nested(RootNested),
        Tuple(RootTuple),
    }
}
mod packed_product_variants {
    use super::*;
    #[allow(dead_code)]
    struct RootRecord {
        value: u8,
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum Root {
        Pair((u8, u8)),
        Record(RootRecord),
        Single(u8),
    }
}
mod tuple_declarations {
    use super::*;
    #[allow(dead_code)]
    struct Root(u8, Existing);
    #[allow(dead_code)]
    struct FieldsEmpty();
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::clone::Clone for FieldsEmpty {
        #[inline]
        fn clone(&self) -> FieldsEmpty {
            FieldsEmpty()
        }
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum ExactChoice {
        A,
        B,
    }
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::clone::Clone for ExactChoice {
        #[inline]
        fn clone(&self) -> ExactChoice {
            match self {
                ExactChoice::A => ExactChoice::A,
                ExactChoice::B => ExactChoice::B,
            }
        }
    }
    #[allow(dead_code)]
    struct Exact(u8, ExactChoice);
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::clone::Clone for Exact {
        #[inline]
        fn clone(&self) -> Exact {
            Exact(
                ::core::clone::Clone::clone(&self.0),
                ::core::clone::Clone::clone(&self.1),
            )
        }
    }
    #[allow(dead_code)]
    pub struct Fields(FieldsEmpty, Exact);
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::clone::Clone for Fields {
        #[inline]
        fn clone(&self) -> Fields {
            Fields(
                ::core::clone::Clone::clone(&self.0),
                ::core::clone::Clone::clone(&self.1),
            )
        }
    }
    #[allow(dead_code)]
    struct HolderPair(u8, u16);
    #[allow(dead_code)]
    struct ExactPair(u8, u16);
    #[allow(dead_code)]
    struct HolderBoxed(u8);
    #[allow(dead_code)]
    struct Holder {
        pair: HolderPair,
        exact: ExactPair,
        boxed: ::std::boxed::Box<HolderBoxed>,
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    pub(crate) enum ChoiceConfigured {
        A,
        B,
    }
    #[allow(dead_code)]
    struct ConfiguredInner(u8);
    #[allow(dead_code)]
    struct Configured(pub(crate) ChoiceConfigured, pub(crate) ConfiguredInner);
}
mod configuration_global_and_local {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    pub enum LeafBranchRoot {
        A,
        B,
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    pub enum BranchRoot {
        Leaf(LeafBranchRoot),
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum RootRegular {
        A,
        B,
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum Root {
        Branch(BranchRoot),
        Regular(RootRegular),
    }
}
mod forwarded_attributes_as_invocation_defaults {
    use super::*;
    #[allow(dead_code)]
    pub struct Public {
        pub value: u8,
    }
    #[allow(dead_code)]
    struct Private {
        value: u8,
    }
}
mod configuration_on_fields {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    pub(crate) enum NestedChoiceHolder {
        A,
        B,
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    pub(crate) enum ChoiceHolder {
        Nested(NestedChoiceHolder),
    }
    #[allow(dead_code)]
    struct Holder {
        pub value: ChoiceHolder,
    }
}
mod outer_attributes_propagate {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    #[repr(C)]
    enum RootChild {
        A,
        B,
    }
    #[allow(dead_code)]
    #[repr(C)]
    struct Root {
        child: RootChild,
    }
}
mod documentation_stays_local {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    ///child only
    enum RootChild {
        A,
        B,
    }
    #[allow(dead_code)]
    ///root only
    struct Root {
        ///field only
        child: RootChild,
    }
}
mod underive_reduces_the_inherited_branch {
    use super::*;
    #[allow(dead_code)]
    struct RootChildLeaf {
        value: u8,
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::fmt::Debug for RootChildLeaf {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "RootChildLeaf",
                "value",
                &&self.value,
            )
        }
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::marker::StructuralPartialEq for RootChildLeaf {}
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::cmp::PartialEq for RootChildLeaf {
        #[inline]
        fn eq(&self, other: &RootChildLeaf) -> bool {
            self.value == other.value
        }
    }
    #[allow(dead_code)]
    struct RootChild {
        leaf: RootChildLeaf,
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::fmt::Debug for RootChild {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "RootChild",
                "leaf",
                &&self.leaf,
            )
        }
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::marker::StructuralPartialEq for RootChild {}
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::cmp::PartialEq for RootChild {
        #[inline]
        fn eq(&self, other: &RootChild) -> bool {
            self.leaf == other.leaf
        }
    }
    #[allow(dead_code)]
    struct Root {
        child: RootChild,
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::fmt::Debug for Root {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "Root",
                "child",
                &&self.child,
            )
        }
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::clone::Clone for Root {
        #[inline]
        fn clone(&self) -> Root {
            Root {
                child: ::core::clone::Clone::clone(&self.child),
            }
        }
    }
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::marker::StructuralPartialEq for Root {}
    #[automatically_derived]
    #[allow(dead_code)]
    impl ::core::cmp::PartialEq for Root {
        #[inline]
        fn eq(&self, other: &Root) -> bool {
            self.child == other.child
        }
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum VariantsBranchLeaf {
        A,
        B,
    }
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::fmt::Debug for VariantsBranchLeaf {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    VariantsBranchLeaf::A => "A",
                    VariantsBranchLeaf::B => "B",
                },
            )
        }
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum VariantsBranch {
        Leaf(VariantsBranchLeaf),
    }
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::fmt::Debug for VariantsBranch {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                VariantsBranch::Leaf(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Leaf",
                        &__self_0,
                    )
                }
            }
        }
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum Variants {
        Branch(VariantsBranch),
    }
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::fmt::Debug for Variants {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Variants::Branch(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Branch",
                        &__self_0,
                    )
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::clone::Clone for Variants {
        #[inline]
        fn clone(&self) -> Variants {
            match self {
                Variants::Branch(__self_0) => {
                    Variants::Branch(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum PayloadLeaf {
        A,
        B,
    }
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::fmt::Debug for PayloadLeaf {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    PayloadLeaf::A => "A",
                    PayloadLeaf::B => "B",
                },
            )
        }
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum Payload {
        Leaf(PayloadLeaf),
    }
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::fmt::Debug for Payload {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Payload::Leaf(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Leaf",
                        &__self_0,
                    )
                }
            }
        }
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum Exact {
        Branch(Payload),
    }
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::fmt::Debug for Exact {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Exact::Branch(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Branch",
                        &__self_0,
                    )
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::clone::Clone for Exact {
        #[inline]
        fn clone(&self) -> Exact {
            match self {
                Exact::Branch(__self_0) => {
                    Exact::Branch(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
}
mod inlined_struct_variants_keep_declaration_attributes {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    #[repr(C)]
    enum RootChild {
        A,
        B,
    }
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::fmt::Debug for RootChild {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    RootChild::A => "A",
                    RootChild::B => "B",
                },
            )
        }
    }
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::marker::StructuralPartialEq for RootChild {}
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::cmp::PartialEq for RootChild {
        #[inline]
        fn eq(&self, other: &RootChild) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
        }
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    #[repr(C)]
    enum Root {
        Branch { child: RootChild },
    }
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::fmt::Debug for Root {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Root::Branch { child: __self_0 } => {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "Branch",
                        "child",
                        &__self_0,
                    )
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::marker::StructuralPartialEq for Root {}
    #[automatically_derived]
    #[allow(dead_code, clippy::enum_variant_names)]
    impl ::core::cmp::PartialEq for Root {
        #[inline]
        fn eq(&self, other: &Root) -> bool {
            match (self, other) {
                (Root::Branch { child: __self_0 }, Root::Branch { child: __arg1_0 }) => {
                    __self_0 == __arg1_0
                }
            }
        }
    }
}
mod cfg_guards_declarations_and_constructors {
    use super::*;
}
mod direct_and_transitive_constructors {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    enum ExprUnary {
        PrefExprUnary(Pref),
        PostExprUnary(Post),
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum Expr {
        Unary(ExprUnary),
        BinExpr(Bin),
        LitStr(String),
        Null,
    }
}
mod inclusions_flatten_once_per_path {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    enum ABCD {
        E,
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum ABC {
        D(ABCD),
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum AB {
        C(ABC),
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum A {
        B(AB),
    }
    ///Injects the `B` constructor occurrence into `A`.
    #[allow(dead_code, non_snake_case)]
    fn AB(value: AB) -> A {
        A::B(value)
    }
    ///Injects the `B::C` constructor occurrence into `A`.
    #[allow(dead_code, non_snake_case)]
    fn ABC(value: ABC) -> A {
        A::B(AB::C(value))
    }
    ///Injects the `B::C::D` constructor occurrence into `A`.
    #[allow(dead_code, non_snake_case)]
    fn ABCD(value: ABCD) -> A {
        A::B(AB::C(ABC::D(value)))
    }
    ///Injects the `B::C::D::E` constructor occurrence into `A`.
    #[allow(dead_code, non_snake_case)]
    fn ABCDE() -> A {
        A::B(AB::C(ABC::D(ABCD::E)))
    }
}
mod inclusions_per_enum_branch {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    enum RootEnabled {
        Leaf,
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum RootDisabled {
        Leaf,
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum Root {
        Enabled(RootEnabled),
        Disabled(RootDisabled),
    }
    ///Injects the `Enabled` constructor occurrence into `Root`.
    #[allow(dead_code, non_snake_case)]
    fn RootEnabled(value: RootEnabled) -> Root {
        Root::Enabled(value)
    }
    ///Injects the `Enabled::Leaf` constructor occurrence into `Root`.
    #[allow(dead_code, non_snake_case)]
    fn RootEnabledLeaf() -> Root {
        Root::Enabled(RootEnabled::Leaf)
    }
}
mod inclusion_names_reverse_incrementally {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    enum BranchRoot {
        Leaf,
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum Root {
        Branch(BranchRoot),
    }
    ///Injects the `Branch` constructor occurrence into `Root`.
    #[allow(dead_code, non_snake_case)]
    fn BranchRoot(value: BranchRoot) -> Root {
        Root::Branch(value)
    }
    ///Injects the `Branch::Leaf` constructor occurrence into `Root`.
    #[allow(dead_code, non_snake_case)]
    fn LeafBranchRoot() -> Root {
        Root::Branch(BranchRoot::Leaf)
    }
}
mod inclusions_carry_variant_conditions {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    enum Root {
        Visible(String),
    }
    ///Injects the `Visible` constructor occurrence into `Root`.
    #[allow(dead_code, non_snake_case)]
    fn RootVisible(value: String) -> Root {
        Root::Visible(value)
    }
}
mod inclusions_stop_at_product_boundaries {
    use super::*;
    #[allow(dead_code, clippy::enum_variant_names)]
    enum RootChild {
        Leaf,
    }
    ///Injects the `Leaf` constructor occurrence into `RootChild`.
    #[allow(dead_code, non_snake_case)]
    fn RootChildLeaf() -> RootChild {
        RootChild::Leaf
    }
    #[allow(dead_code, clippy::enum_variant_names)]
    enum Root {
        Branch { child: RootChild },
    }
    ///Injects the `Branch` constructor occurrence into `Root`.
    #[allow(dead_code, non_snake_case)]
    fn RootBranch(value: (RootChild,)) -> Root {
        Root::Branch { child: value.0 }
    }
}
mod visibility_inherited_through_the_branch {
    use super::*;
    #[allow(dead_code)]
    pub(crate) struct InheritedChildLeaf {
        pub(crate) value: u8,
    }
    #[allow(dead_code)]
    pub(crate) struct InheritedChild {
        pub(crate) nested: InheritedChildLeaf,
    }
    #[allow(dead_code)]
    pub(crate) struct Inherited {
        pub(crate) field: InheritedChild,
    }
    #[allow(dead_code)]
    struct WrittenChildLeaf {
        value: u8,
    }
    #[allow(dead_code)]
    struct WrittenChild {
        nested: WrittenChildLeaf,
    }
    #[allow(dead_code)]
    pub struct Written {
        pub field: WrittenChild,
    }
    #[allow(dead_code)]
    struct OverriddenChild {
        pub value: u8,
    }
    #[allow(dead_code)]
    pub struct Overridden {
        field: OverriddenChild,
    }
}
