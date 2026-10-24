//! Documentation preservation and optional missing-documentation allowances.

#![deny(missing_docs)]

use these_macros_should_be_illegal::{macro_args, strutuct};

strutuct! {
    /// A declaration documented by its caller.
    pub Documented {
        /// A field documented by its caller.
        pub value: u8,
    }
}

#[macro_args(pub, undoc, inc)]
strutuct! {
    Undocumented {
        Unit,
        Tuple(u8, u16),
        Named { value: u8 },
        Nested(Nested { Leaf }),
        Marker |UnitPayload|,
    }
}

strutuct! {
    #[strutuct(pub, allow_missing_docs)]
    UndocumentedStruct {
        child: Child { value: u8 },
        tuple: Pair(u8, u16),
    }
}

strutuct! {
    #[strutuct(pub, no_undoc)]
    /// Only one field permits missing documentation.
    FieldAllowance {
        #[strutuct(undoc)]
        pub value: u8,
    }
}

/// The flag covers hoisted declarations, fields, variants, and inclusion helpers.
#[test]
fn permits_undocumented_generated_families() {
    let root = UndocumentedStruct {
        child: UndocumentedStructChild { value: 3 },
        tuple: UndocumentedStructPair(4, 5),
    };
    assert_eq!((root.child.value, root.tuple.0, root.tuple.1), (3, 4, 5));
    assert!(matches!(UndocumentedUnit(), Undocumented::Unit));
    assert!(matches!(UndocumentedNestedLeaf(), Undocumented::Nested(_)));
    assert_eq!(FieldAllowance { value: 7 }.value, 7);
    assert_eq!(Documented { value: 8 }.value, 8);
}
