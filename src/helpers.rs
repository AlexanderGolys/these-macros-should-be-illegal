//! Reusable macro plumbing and public helpers for composing transformations.

/// Implements the attribute that excludes selected macro inputs from rewriting.
mod excluded_macros;
/// Implements out-of-line module loading and macro injection.
mod expand;
/// Implements forwarding outer attributes into function-like macro inputs.
mod forward_attributes;
/// Provides shared recursive token preprocessing and configuration handling.
pub(crate) mod preprocessing;

/// Adds macro names to the preprocessing exclusion set.
pub(crate) use excluded_macros::excluded_macros as excluded_macros_impl;
/// Expands an out-of-line module with injected function-like macros.
pub(crate) use expand::expand as expand_impl;
/// Moves invocation attributes into an opaque function-like macro input.
pub(crate) use forward_attributes::forward_attributes as forward_attributes_impl;
