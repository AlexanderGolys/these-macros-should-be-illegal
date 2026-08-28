//! Shape-aware macros providing local syntax extensions or replacement grammars.

/// Implements the `discriminated_str` enum attribute.
mod discriminated_str;
/// Implements nested algebraic declarations for `strutuct!` and `emmun!`.
mod strutuct;

/// Generates an enum accessor from string-like discriminants.
pub(crate) use discriminated_str::discriminated_str as discriminated_str_impl;
/// Lowers one nested algebraic declaration family.
pub(crate) use strutuct::strutuct as strutuct_impl;
