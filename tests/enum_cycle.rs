//! Constant cyclic successor behavior.
use these_macros_should_be_illegal::enum_cycle;

#[enum_cycle(pub next)]
#[derive(Debug, PartialEq)]
enum Phase {
    First = 7,
    #[cfg(any())]
    Hidden,
    #[cfg_attr(all(), cfg(any()))]
    AlsoHidden,
    Last = 42,
}

const WRAPPED: Phase = Phase::Last.next();

#[test]
fn cycles_in_enabled_declaration_order() {
    assert_eq!(WRAPPED, Phase::First);
    assert_eq!(Phase::First.next(), Phase::Last);
    assert_eq!(Phase::First.next().next(), Phase::First);
    assert_eq!(Phase::Last as u8, 42);
}

#[test]
fn supports_local_singleton_without_copy() {
    #[enum_cycle(#[inline] pub(crate) next,)]
    enum Only {
        Value,
    }
    const VALUE: Only = Only::Value.next();
    assert!(matches!(VALUE.next(), Only::Value));
}

use these_macros_should_be_illegal::{discriminated_str, enum_fn};

#[enum_cycle(next)]
#[discriminated_str(label)]
enum CycleOutsideStrings {
    First = "first",
    Last = "last",
}

#[discriminated_str(label)]
#[enum_cycle(next)]
enum StringsOutsideCycle {
    First = "first",
    Last = "last",
}

#[enum_cycle(next)]
#[enum_fn(const code -> u8)]
enum CycleOutsideMapping {
    First = 10,
    Last = 20,
}

#[enum_fn(const code -> u8)]
#[enum_cycle(next)]
enum MappingOutsideCycle {
    First = 10,
    Last = 20,
}

#[test]
fn composes_with_variant_mappings_in_both_orders() {
    const CODE: u8 = CycleOutsideMapping::Last.next().code();
    const OTHER_CODE: u8 = MappingOutsideCycle::Last.next().code();
    assert_eq!((CODE, OTHER_CODE), (10, 10));
    assert_eq!(CycleOutsideStrings::Last.next().label(), "first");
    assert_eq!(StringsOutsideCycle::Last.next().label(), "first");
    assert_eq!(CycleOutsideStrings::First.next().label(), "last");
    assert_eq!(StringsOutsideCycle::First.next().label(), "last");
}

#[enum_cycle(next)]
#[derive(Debug, PartialEq)]
enum Conditional {
    #[cfg(all())]
    First,
    #[cfg_attr(all(), cfg(all()))]
    Last,
}

#[test]
fn cycles_when_every_variant_has_conditional_attributes() {
    assert_eq!(Conditional::First.next(), Conditional::Last);
    assert_eq!(Conditional::Last.next(), Conditional::First);
}
