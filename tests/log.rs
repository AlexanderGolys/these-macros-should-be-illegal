//! Consumer tests for the enum of a struct's field names and its evaluation map.

use std::collections::HashMap;

use these_macros_should_be_illegal::{Log, forall, qf};

/// Style one highlight group is drawn with.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Style(u8);

/// Styles of the groups a theme is written in.
#[derive(Log)]
#[allow(non_snake_case)]
pub struct BaseGroups {
    /// Style of a keyword.
    pub Keyword: Style,
    /// Style of a function name.
    pub Function: Style,
    /// Style of the background.
    pub Bg: Style,
    /// Style of ordinary text.
    pub Text: Style,
}

/// Styles whose fields one prefix writes before the derive reads them.
#[forall(#[doc = "Style of one group."] pub, Style)]
#[derive(Log)]
#[log(pub Group)]
#[allow(non_snake_case)]
pub struct Theme(Keyword, Function, Bg);

/// Point of the plane, indexed by its axes.
#[derive(Log)]
#[log(pub enum Axis)]
pub struct Point {
    /// Distance along the first axis.
    pub x: f32,
    /// Distance along the second axis.
    pub y: f32,
}

/// Record whose fields hold different types.
#[derive(Log)]
pub struct Record {
    /// Number of entries.
    pub count: u8,
    /// Name of the record.
    pub name: &'static str,
}

/// Pair borrowing both of its fields.
#[derive(Log)]
pub struct Pair<'a, T: Copy>
where
    T: Default,
{
    /// First borrowed value.
    pub left: &'a T,
    /// Second borrowed value.
    pub right: &'a T,
}

/// Struct whose field names are keywords.
#[derive(Log)]
pub struct Keywords {
    /// Field named by a keyword.
    pub r#type: u8,
    /// Field named by another keyword.
    pub r#fn: u8,
}

/// Struct whose shared type is produced by another macro.
#[derive(Log)]
pub struct Produced {
    /// Values of the first bucket.
    pub small: qf!(Vec<u8>),
    /// Values of the second bucket.
    pub large: qf!(Vec<u8>),
}

/// Struct with nothing to name.
#[derive(Log)]
pub struct Nothing;

/// Struct whose second field is compiled in conditionally.
#[derive(Log)]
pub struct Conditional {
    /// Field that is always compiled.
    pub always: u8,
    /// Field that is compiled under a condition that holds.
    #[cfg(test)]
    pub sometimes: u8,
}

/// Struct deriving both this index set and the ordinary traits.
#[derive(Log, Clone, Copy, PartialEq, Debug)]
pub struct Beside {
    /// First coordinate.
    pub first: u8,
    /// Second coordinate.
    pub second: u8,
}

/// Declares one struct whose field types are forwarded by its caller.
macro_rules! shared_record {
    ($name:ident { $($field:ident : $shared:ty),* $(,)? }) => {
        /// Struct generated with forwarded field types.
        #[derive(Log)]
        pub struct $name {
            $(
                /// Forwarded field.
                pub $field: $shared,
            )*
        }
    };
}

shared_record!(Forwarded {
    first: u32,
    second: u32
});

/// Reading a field name on a value is the evaluation map of the exponential object.
#[test]
fn evaluates_a_field_name_on_the_struct() {
    let groups = BaseGroups {
        Keyword: Style(1),
        Function: Style(2),
        Bg: Style(3),
        Text: Style(4),
    };

    assert_eq!(groups[LogBaseGroups::Keyword], Style(1));
    assert_eq!(groups[LogBaseGroups::Text], Style(4));
}

/// The evaluation map is writable through the mutable indexing operator.
#[test]
fn writes_through_a_field_name() {
    let mut point = Point { x: 0.0, y: 0.0 };

    point[Axis::x] = 1.5;
    point[Axis::y] = point[Axis::x] * 2.0;

    assert_eq!((point.x, point.y), (1.5, 3.0));
}

/// Every field name is an ordinary value, so a struct is a map out of the enum.
#[test]
fn reads_the_struct_as_a_map_out_of_the_enum() {
    let groups = BaseGroups {
        Keyword: Style(1),
        Function: Style(2),
        Bg: Style(3),
        Text: Style(4),
    };
    let names = [
        LogBaseGroups::Keyword,
        LogBaseGroups::Function,
        LogBaseGroups::Bg,
        LogBaseGroups::Text,
    ];
    let evaluated: HashMap<LogBaseGroups, Style> =
        names.into_iter().map(|name| (name, groups[name])).collect();

    assert_eq!(evaluated[&LogBaseGroups::Bg], Style(3));
    assert_eq!(evaluated.len(), 4);
}

/// An attribute macro on the struct runs before the derive reads its fields.
#[test]
fn names_the_fields_one_prefix_writes() {
    let mut theme = Theme {
        Keyword: Style(1),
        Function: Style(2),
        Bg: Style(3),
    };
    theme[Group::Bg] = Style(7);

    assert_eq!(theme[Group::Function], Style(2));
    assert_eq!(theme[Group::Bg], Style(7));
}

/// Fields holding different types name the index set of a genuine product.
#[test]
fn names_the_fields_of_a_product() {
    let record = Record {
        count: 2,
        name: "pair",
    };

    assert_eq!(record.count, 2);
    assert_ne!(LogRecord::count, LogRecord::name);
}

/// The generics and bounds of the struct reach its indexing operator.
#[test]
fn evaluates_a_generic_struct() {
    let pair = Pair {
        left: &2_u8,
        right: &3_u8,
    };

    assert_eq!(pair[LogPair::left] + pair[LogPair::right], 5);
}

/// Keyword field names are evaluated through the variants spelled like them.
#[test]
fn evaluates_raw_identifier_fields() {
    let keywords = Keywords { r#type: 1, r#fn: 2 };

    assert_eq!(
        keywords[LogKeywords::r#type] + keywords[LogKeywords::r#fn],
        3
    );
}

/// A shared type written as a macro call is indexed like any other.
#[test]
fn evaluates_a_generated_type() {
    let produced = Produced {
        small: vec![1],
        large: vec![2, 3],
    };

    assert_eq!(
        produced[LogProduced::small].len() + produced[LogProduced::large].len(),
        3
    );
}

/// A struct without fields is the exponent of the uninhabited enum.
#[test]
fn names_no_field_of_a_unit_struct() {
    let nothing = Nothing;
    let names: Vec<LogNothing> = Vec::new();

    assert_eq!(std::mem::size_of_val(&nothing), 0);
    assert!(names.is_empty());
}

/// A field compiled in conditionally keeps its condition on the variant naming it.
#[test]
fn names_a_conditional_field() {
    let conditional = Conditional {
        always: 1,
        sometimes: 2,
    };

    assert_eq!(
        conditional[LogConditional::always] + conditional[LogConditional::sometimes],
        3
    );
}

/// The derive stands beside the ordinary ones, which still see the struct.
#[test]
fn derives_beside_the_ordinary_traits() {
    let beside = Beside {
        first: 1,
        second: 2,
    };

    assert_eq!(beside, beside.clone());
    assert_eq!(beside[LogBeside::first] + beside[LogBeside::second], 3);
}

/// Field types forwarded by a declarative macro are compared like written ones.
#[test]
fn evaluates_forwarded_field_types() {
    let forwarded = Forwarded {
        first: 1,
        second: 2,
    };

    assert_eq!(
        forwarded[LogForwarded::first] + forwarded[LogForwarded::second],
        3
    );
}
