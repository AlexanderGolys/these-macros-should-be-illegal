//! Shape-aware macros providing local syntax extensions or replacement grammars.

/// Implements structural callable aliases and local call macros.
mod callable;
/// Implements operators that follow from the ones a type already has.
mod complete_ops;
/// Implements the `discriminated_str` enum attribute.
mod discriminated_str;
/// Implements the `enum_fn` enum attribute.
mod enum_fn;
/// Implements ownership and assignment overloads of one operator.
mod operators;
/// Implements compile-time name and type stringification.
pub(crate) mod stringify;
/// Implements nested algebraic declarations for `strutuct!` and `emmun!`.
mod strutuct;
/// Adds the reserved callable alias to one selected trait method.
pub(crate) use callable::callable as callable_impl;
/// Creates a local value and its same-name invocation macro.
pub(crate) use callable::make_fn as make_fn_impl;
/// Completes the operators that follow from a type's existing ones.
pub(crate) use complete_ops::complete_ops as complete_ops_impl;
/// Generates unique string discriminants and constructors for an enum.
pub(crate) use discriminated_str::discriminated_str as discriminated_str_impl;
/// Generates an enum method from per-variant match expressions.
pub(crate) use enum_fn::enum_fn as enum_fn_impl;
/// Repeats one operator on references across its owned operand forms.
pub(crate) use operators::overload_op as overload_op_impl;
/// Case selectors and the shared name conversion implementation.
pub(crate) use stringify::{
    Case as StringCase, stringify_case, stringify_type as stringify_type_impl,
};
/// Lowers one nested algebraic declaration family.
pub(crate) use strutuct::strutuct as strutuct_impl;
