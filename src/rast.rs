//! Nearly ordinary Rust conveniences that preserve the surrounding grammar.

/// Implements qualification of common unqualified standard-library types.
mod qf;

/// Qualifies common types in one ordinary Rust type fragment.
pub(crate) use qf::qualify_common as qualify_common_impl;
