//! Procedural-macro implementations and their shared token preprocessing.

/// Implements the `str_disc` attribute macro.
mod discriminated_str;
/// Implements the `excluded_macros` attribute macro.
mod excluded_macros;
/// Implements the `expand` function-like macro for out-of-line modules.
mod expand;
/// Implements the `literally_literal_string` function-like macro.
mod literally_literal_string;
/// Provides shared recursive token preprocessing and configuration handling.
mod preprocessing;
/// Implements nested algebraic declarations for the `strutuct` macro.
mod strutuct;

/// Generates an enum accessor from string discriminants.
pub(super) use discriminated_str::str_disc as str_disc_impl;
/// Adds macro names to the preprocessing exclusion set.
pub(super) use excluded_macros::excluded_macros as excluded_macros_impl;
/// Expands an out-of-line module with injected function-like macros.
pub(super) use expand::expand as expand_impl;
/// Rewrites the extended owned-string syntax.
pub(super) use literally_literal_string::literally_literal_string as literally_literal_string_impl;
/// Lowers one nested struct or enum declaration.
pub(super) use strutuct::strutuct as strutuct_impl;
