//! Transformations whose inputs and outputs are themselves macro syntax.

/// Implements construction and reflection of function-like macro invocations.
mod invocation;
/// Implements finite permutations of comma-separated token trees.
mod permutation;

/// Constructs one function-like macro invocation around an opaque body.
pub(crate) use invocation::invoke;
/// Reflects two nested macro invocations around an opaque body.
pub(crate) use invocation::reflect as reflect_impl;
/// Applies cycle notation to the leading token trees of a stream.
pub(crate) use permutation::perm as perm_impl;
