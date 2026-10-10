//! Experimental procedural macros for compact local DSLs, recursive token
//! rewriting, and transformations of macro structure.
//!
//! The main entry points are [`enum_fn`] for per-variant methods,
//! [`discriminated_str`] for reversible string discriminants, [`strutuct`] for
//! nested algebraic declarations, and [`expand`] for applying recursive syntax
//! extensions to an out-of-line module. Every macro is independent; import only
//! the syntax used by the caller.
//!
//! ```
//! # use these_macros_should_be_illegal::enum_fn;
//!
//! #[enum_fn(code: usize)]
//! enum Status {
//!     Ready = 200,
//!     Missing,
//! }
//!
//! assert_eq!(Status::Ready.code(), Some(200));
//! assert_eq!(Status::Missing.code(), None);
//! ```
//!
//! The [crate repository](https://github.com/AlexanderGolys/these-macros-should-be-illegal)
//! contains the complete book and runnable examples.

use proc_macro::TokenStream;

#[macro_use]
mod lib_mbe;

macro_cat_mod! { dust
    //! Whole-stream syntax extensions applied recursively before ordinary parsing.
    literally_literal_string;
    shared_match_arms;
}

attr_macro_cat_mod! { attr
    //! Attribute macros extending the items they are applied to.
    callable;
    discriminated_str;
    enum_fn;
    overload_op;
    complete_ops;
    excluded_macros;
    forward_attributes;
}

macro_cat_mod! { generate
    //! Generators of compound structures with a fixed layout.
    strutuct [emmun];
}

macro_cat_mod! { rast
    //! Remaining conveniences over ordinary Rust syntax.
    qf;
    stringify_as;
    make_fn;
}

macro_cat_mod! { meta
    //! Macros transforming other macros.
    perm;
    reflect;
    expand;
}

/// Internal plumbing shared by the macro implementations.
mod helpers {
    pub mod arguments;
    pub mod fresh_name;
    pub mod preprocessing;
    pub mod ungroup;
}
