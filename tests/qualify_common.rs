//! Consumer tests for the `qualify_common` function-like macro.

use std::{collections::HashMap, sync::Arc};

use these_macros_should_be_illegal::qf;

/// Ensures common unqualified paths are rewritten recursively.
#[test]
fn qualifies_common_types_recursively() {
    let value: qf!(Option<Vec<Arc<String>>>) = Some(vec![Arc::new(String::from("text"))]);
    assert!(value.is_some());
}

/// Ensures deliberately qualified paths remain distinct.
#[test]
fn preserves_already_qualified_paths() {
    mod application {
        /// Application-specific type deliberately named like the standard collection.
        pub struct HashMap;
    }

    let _: qf!(application::HashMap) = application::HashMap;
    let _: qf!(HashMap<String, usize>) = HashMap::new();
}
