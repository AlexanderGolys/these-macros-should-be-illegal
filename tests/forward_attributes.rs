//! Consumer tests for forwarding attributes into function-like macro inputs.

use these_macros_should_be_illegal::{forward_attributes, strutuct};

#[forward_attributes]
#[derive(Debug, PartialEq)]
strutuct! {
    DirectlyForwarded { Ready, Waiting }
}

#[doc = "An attribute written outside the forwarding attribute."]
#[forward_attributes]
#[derive(Debug, PartialEq)]
strutuct! {
    OutsideDocumented { Present, Missing }
}

#[forward_attributes]
#[strutuct(product_variants = false)]
strutuct! {
    ConfiguredThroughAttributes
    Pair(u8, u8)
}

/// Direct forwarding makes ordinary derives apply to the generated family.
#[test]
fn forwards_direct_invocation_attributes() {
    assert_eq!(DirectlyForwarded::Ready, DirectlyForwarded::Ready);
    assert_eq!(format!("{:?}", DirectlyForwarded::Waiting), "Waiting");
}

/// Inert attributes outside the forwarding attribute are retained and moved too.
#[test]
fn forwards_attributes_written_outside_the_forwarder() {
    assert_eq!(OutsideDocumented::Present, OutsideDocumented::Present);
}

/// Forwarded configuration controls the receiving function-like macro.
#[test]
fn forwards_macro_configuration() {
    let pair = ConfiguredThroughAttributes::Pair(3, 4);

    assert!(matches!(pair, ConfiguredThroughAttributes::Pair(3, 4)));
}
