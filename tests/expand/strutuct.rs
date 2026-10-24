#![no_main]

use these_macros_should_be_illegal::strutuct;

pub struct Existing;
pub struct Delimited<T>(pub T);
pub struct Pref(pub u8);
pub struct Post(pub u8);
pub struct Bin(pub u8);

macro_rules! type_macro {
    ($($tokens:tt)*) => {
        u8
    };
}

mod nested_enum_in_struct_field {
    use super::*;

    strutuct! {
        S a: A { A1, A2, A3 }, b: Existing,
    }
}

mod local_derive_extends_the_nested_branch {
    use super::*;

    strutuct! {
        #[derive(Clone, Default)] Root #[derive(Debug)] branch: Branch { leaf: Leaf { value: String } },
    }
}

mod rust_types_stay_verbatim {
    use super::*;

    strutuct! {
        Root
        map: std::collections::HashMap<fn() -> u8, Box<dyn Fn(u8) -> u8>>,
        pointer: fn(u8) -> u8,
        array: [u8; 2],
        next: u8,
    }
}

mod relative_names_across_type_contexts {
    use super::*;

    strutuct! {
        Root
        direct: Direct { A, B },
        optional: Optional { A, B }?,
        boxed: Boxed { A, B }*,
        tuple: Tuple(u8, Element { A, B }),
        exact: |Exact| { child: Child { A, B } },
        existing: Existing,
    }
}

mod postfix_type_operators {
    use super::*;

    strutuct! {
        S optional: Existing?, nested: Existing?*,
    }
}

mod type_macro_inputs_stay_opaque {
    use super::*;

    strutuct! {
        Root value: Delimited<type_macro!(Content { Empty })>,
    }
}

mod braced_keyword_declarations_and_visibility {
    use super::*;

    strutuct! {
        pub(crate) struct Product { priv hidden: u8, pub shown: u8, }
    }

    strutuct! {
        struct Private { field: u8 }
    }
}

mod keywords_on_exact_payloads {
    use super::*;

    strutuct! {
        Root { enum |Choice| { A, B }, struct Named |Marker| }
    }
}

mod rust_variant_shapes {
    use super::*;

    strutuct! {
        Root {
            Unit,
            EmptyTuple(),
            Single(u8),
            Pair(u8, Existing),
            EmptyRecord {},
            Record { value: u8, kind: Kind { A, B } },
            Nested(Nested { Leaf }),
            Tuple(Tuple(u8, u8)),
        }
    }
}

mod packed_product_variants {
    use super::*;

    strutuct! {
        #[strutuct(product_variants)]
        Root { Pair(u8, u8), Record { value: u8 }, Single(u8) }
    }
}

mod tuple_declarations {
    use super::*;

    strutuct! {
        Root(u8, Existing)
    }

    strutuct! {
        #[derive(Clone)] pub struct Fields(priv Empty(), |Exact|(u8, Choice { A, B }))
    }

    strutuct! {
        Holder
        pair: Pair(u8, u16),
        exact: |ExactPair|(u8, u16),
        boxed: Boxed(u8)*,
    }

    strutuct! {
        Configured(
            #[strutuct(pub(crate), reverse_concat)] Choice { A, B },
            pub(crate) priv Inner(u8),
        )
    }
}

mod configuration_global_and_local {
    use super::*;

    strutuct! {
        #[strutuct(priv)]
        enum Root {
            #[strutuct(pub, reverse_concat)]
            Branch(Branch { Leaf(Leaf { A, B }) }),
            Regular(Regular { A, B }),
        }
    }
}

mod forwarded_attributes_as_invocation_defaults {
    use super::*;

    strutuct! {
        #[strutuct(pub)] ; struct Public { value: u8 }
    }

    strutuct! {
        #[strutuct(pub)] ; #[strutuct(priv)] struct Private { value: u8 }
    }
}

mod configuration_on_fields {
    use super::*;

    strutuct! {
        struct Holder {
            #[strutuct(pub(crate), reverse_concat)]
            pub value: enum Choice { Nested(Nested { A, B }) },
        }
    }
}

mod outer_attributes_propagate {
    use super::*;

    strutuct! {
        #[repr(C)] struct Root { child: enum Child { A, B }, }
    }
}

mod documentation_stays_local {
    use super::*;

    strutuct! {
        #[doc = "root only"]
        Root
        #[doc = "field only"]
        child: #[doc = "child only"] Child { A, B },
    }
}

mod underive_reduces_the_inherited_branch {
    use super::*;

    strutuct! {
        #[derive(Debug, Clone, Eq, PartialEq)] #[underive(Eq)]
        Root #[underive(Clone, Hash)] child: Child { leaf: Leaf { value: u8 } },
    }

    strutuct! {
        #[derive(Debug, Clone)] Variants #[underive(Clone)] Branch(Branch { Leaf(Leaf { A, B }) })
    }

    strutuct! {
        #[derive(Debug, Clone)] Exact #[underive(Clone)] Branch |Payload| { Leaf(Leaf { A, B }) }
    }
}

mod inlined_struct_variants_keep_declaration_attributes {
    use super::*;

    strutuct! {
        #[derive(Debug, PartialEq)] #[repr(C)] #[strutuct(no_product_variants)]
        Root Branch { child: Child { A, B } }
    }
}

mod cfg_guards_declarations_and_constructors {
    use super::*;

    strutuct! {
        #[cfg(any())] Root Branch(Branch { A, B })
    }
}

mod direct_and_transitive_constructors {
    use super::*;

    strutuct! {
        Expr Unary(Unary { (Pref), (Post) }) (Bin) LitStr(String) Null
    }
}

mod inclusions_flatten_once_per_path {
    use super::*;

    strutuct! {
        #[strutuct(inclusions)] A B(B { C(C { D(D { E }) }) })
    }
}

mod inclusions_per_enum_branch {
    use super::*;

    strutuct! {
        Root
        #[strutuct(inclusions)] Enabled(Enabled { Leaf })
        #[strutuct(no_inclusions)] Disabled(Disabled { Leaf })
    }
}

mod inclusion_names_reverse_incrementally {
    use super::*;

    strutuct! {
        #[strutuct(inclusions, reverse_concat)] Root Branch(Branch { Leaf })
    }
}

mod inclusions_carry_variant_conditions {
    use super::*;

    strutuct! {
        #[strutuct(inclusions)] Root #[cfg(any())] Hidden(String) Visible(String)
    }
}

mod inclusions_stop_at_product_boundaries {
    use super::*;

    strutuct! {
        #[strutuct(inclusions)] Root Branch { child: Child { Leaf } }
    }
}

mod visibility_inherited_through_the_branch {
    use super::*;

    strutuct! {
        #[strutuct(pub(crate))]
        struct Inherited { field: Child { nested: Leaf { value: u8 } } }
    }

    strutuct! {
        pub struct Written { pub field: Child { nested: Leaf { value: u8 } } }
    }

    strutuct! {
        #[strutuct(pub)]
        struct Overridden { priv field: priv Child { value: u8 } }
    }
}
