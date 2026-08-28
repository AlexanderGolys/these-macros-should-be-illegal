//! Whole-stream function-like syntax extensions applied before ordinary parsing.

/// Implements the recursively rewritten owned-string syntax.
mod literally_literal_string;
/// Implements recursively shared match-arm right-hand sides.
mod shared_match_arms;

/// Rewrites the extended owned-string syntax.
pub(crate) use literally_literal_string::literally_literal_string as literally_literal_string_impl;
/// Rewrites shared match-arm alternatives into independent ordinary arms.
pub(crate) use shared_match_arms::shared_match_arms as shared_match_arms_impl;
