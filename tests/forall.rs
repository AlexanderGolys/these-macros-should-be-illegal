//! Consumer tests for repeating one declaration prefix over fields and variants.

use these_macros_should_be_illegal::{discriminated_str, enum_fn, forall, qf};

/// Tuple struct whose types all take the distributed visibility.
#[forall(pub(crate))]
pub struct Pixel(u8, u8, u8);

/// Tuple positions named by one shared type.
#[forall(pub, f32)]
#[derive(Debug, PartialEq)]
pub struct Point(x, y);

/// Shared type borrowing the item's lifetime.
#[forall(pub, &'a str)]
pub struct Labels<'a>(short, long);

/// Shared array type.
#[forall(pub, [u8; 2])]
pub struct Pairs(first, second);

/// Shared function-pointer type.
#[forall(pub, fn(u8) -> u8)]
pub struct Transforms(incoming, outgoing);

/// Shared raw-pointer type.
#[forall(pub, *const u8)]
pub struct Bounds(start, end);

/// Shared tuple type.
#[forall(pub, (u8, u8))]
pub struct Spans(near, far);

/// Shared generic type under the item's where clause.
#[forall(pub, Vec<T>)]
pub struct Buckets<T>(small, large)
where
    T: Clone;

/// Tuple struct whose prefix is written in reverse of declaration order.
#[forall(u8, pub, #[doc = "One channel of the shared triple."])]
pub struct Channels(red, green, blue);

/// Braced struct whose fields take the distributed visibility.
#[forall(pub)]
#[derive(Debug, PartialEq)]
pub struct Mixed {
    /// Field taking the distributed visibility.
    named: u8,
    /// Field declaring the visibility it keeps.
    pub(crate) explicit: u8,
}

/// Union whose fields take the distributed attribute and visibility.
#[forall(#[doc = "One shared field of a raw view."] pub)]
pub union Raw {
    integer: u32,
    float: f32,
}

/// Enum whose variants take the shared types as unnamed fields.
#[forall(u16, String)]
#[derive(Debug, PartialEq)]
pub enum Response {
    /// Variant holding the shared types alone.
    Redirect,
    /// Variant holding the shared types after its own.
    Error(&'static str),
}

/// Enum whose variants take one shared group of named fields.
#[forall({ code: u16, path: String })]
#[derive(Debug, PartialEq)]
pub enum Request {
    /// Variant holding the shared named fields alone.
    Read,
    /// Variant holding the shared named fields beside its own.
    Write {
        /// Field the variant declares itself.
        payload: u8,
    },
}

/// Enum whose variants take the shared attributes alone.
#[forall(#[doc = "One stage of the pipeline."], #[allow(dead_code)])]
#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum Stage {
    /// Unit variant taking the shared attributes.
    Parse,
    /// Braced variant taking the shared attributes.
    Lower {
        /// Field the variant declares itself.
        depth: usize,
    },
    /// Discriminated variant taking the shared attributes.
    Emit(u8) = 7,
}

/// A distributed visibility reaches every type of a tuple struct.
#[test]
fn distributes_a_visibility_over_tuple_fields() {
    let pixel = Pixel(1, 2, 3);

    assert_eq!((pixel.0, pixel.1, pixel.2), (1, 2, 3));
}

/// One shared type turns tuple positions into named fields.
#[test]
fn names_tuple_fields_sharing_one_type() {
    let point = Point { x: 0.25, y: 0.75 };

    assert_eq!(point, Point { x: 0.25, y: 0.75 });
    assert_eq!(point.x + point.y, 1.0);
}

/// The parts of the prefix reach the fields wherever they are written.
#[test]
fn reads_the_prefix_in_any_order() {
    let channels = Channels {
        red: 1,
        green: 2,
        blue: 3,
    };

    assert_eq!(channels.red + channels.green + channels.blue, 6);
}

/// A shared type may borrow the item's lifetimes.
#[test]
fn shares_a_borrowed_type() {
    let labels = Labels {
        short: "px",
        long: "pixels",
    };

    assert_eq!(labels.short.len() + labels.long.len(), 8);
}

/// Arrays, function pointers, raw pointers and tuples are shared like any type.
#[test]
fn shares_compound_types() {
    let pairs = Pairs {
        first: [1, 2],
        second: [3, 4],
    };
    let transforms = Transforms {
        incoming: u8::swap_bytes,
        outgoing: u8::reverse_bits,
    };
    let bounds = Bounds {
        start: std::ptr::null(),
        end: std::ptr::null(),
    };
    let spans = Spans {
        near: (0, 1),
        far: (2, 3),
    };

    assert_eq!(pairs.first[1] + pairs.second[0], 5);
    assert_eq!((transforms.incoming)(8) + (transforms.outgoing)(1), 136);
    assert_eq!(bounds.start, bounds.end);
    assert_eq!(spans.near.1 + spans.far.0, 3);
}

/// A generic shared type keeps the item's parameters and its where clause.
#[test]
fn shares_a_generic_type_under_a_where_clause() {
    let buckets = Buckets {
        small: vec![1_u8],
        large: vec![2_u8, 3],
    };

    assert_eq!(buckets.small.len() + buckets.large.len(), 3);
}

/// Fields keep the visibility they declare and inherit the one they leave out.
#[test]
fn keeps_an_explicit_field_visibility() {
    let mixed = Mixed {
        named: 1,
        explicit: 2,
    };

    assert_eq!(
        mixed,
        Mixed {
            named: 1,
            explicit: 2
        }
    );
}

/// A union distributes over its named fields like a braced struct.
#[test]
fn distributes_over_union_fields() {
    let raw = Raw { integer: 0 };

    assert_eq!(unsafe { raw.float }, 0.0);
}

/// Shared types become the unnamed fields of every variant.
#[test]
fn appends_shared_types_to_every_variant() {
    let redirect = Response::Redirect(301, String::from("/moved"));
    let error = Response::Error("not found", 404, String::from("/missing"));

    assert_eq!(redirect, Response::Redirect(301, String::from("/moved")));
    assert!(matches!(error, Response::Error("not found", 404, _)));
}

/// One shared group of named fields reaches every variant.
#[test]
fn adds_shared_named_fields_to_every_variant() {
    let read = Request::Read {
        code: 1,
        path: String::from("/etc"),
    };
    let write = Request::Write {
        payload: 7,
        code: 2,
        path: String::from("/tmp"),
    };

    assert_eq!(
        read,
        Request::Read {
            code: 1,
            path: String::from("/etc")
        }
    );
    assert!(matches!(
        write,
        Request::Write {
            payload: 7,
            code: 2,
            ..
        }
    ));
}

/// Shared attributes reach every variant, whatever fields it declares.
#[test]
fn distributes_attributes_over_every_variant() {
    assert_eq!(Stage::Parse, Stage::Parse);
    assert_eq!(Stage::Lower { depth: 2 }, Stage::Lower { depth: 2 });
    assert_eq!(Stage::Emit(1), Stage::Emit(1));
}

mod restricted {
    //! Fields whose visibility is restricted to this module.

    use these_macros_should_be_illegal::forall;

    /// Struct whose fields take a visibility restricted to a path.
    #[forall(pub(in crate::restricted), u8)]
    pub struct Scoped(low, high);

    /// Builds one value where its restricted fields are visible.
    pub fn scoped() -> Scoped {
        Scoped { low: 1, high: 2 }
    }

    /// Reads the restricted fields where they are visible.
    pub fn total(scoped: &Scoped) -> u16 {
        u16::from(scoped.low) + u16::from(scoped.high)
    }
}

/// Fields named by keywords.
#[forall(pub, u8)]
pub struct Keywords(r#type, r#fn, r#struct);

/// Shared type over a lifetime, a const generic and a defaulted parameter.
#[forall(pub, &'a [T; N])]
pub struct Windows<'a, T: Copy = u8, const N: usize = 2>(current, previous)
where
    T: Default;

/// Fields holding boxed trait objects.
#[forall(pub, Box<dyn Fn(u8) -> u8 + Send>)]
pub struct Boxed(twice, thrice);

/// Fields holding higher-ranked function pointers.
#[forall(pub, for<'a> fn(&'a u8) -> &'a u8)]
pub struct Ranked(identity);

/// Field linking to another value of the same type.
#[forall(pub, Option<Box<Self>>)]
pub struct Node(next);

/// Fields whose shared type is produced by another macro.
#[forall(pub, qf!(Vec<u8>))]
pub struct Produced(values, spare);

/// Struct taking its visibility from one prefix and its type from the next.
#[forall(pub)]
#[forall(u8)]
pub struct Stacked(first, second);

/// Struct whose fields take an attribute read by a derive macro.
#[forall(#[serde(default)] pub, u8)]
#[derive(serde::Deserialize)]
pub struct Defaults(low, high);

/// Enum whose variants take their documentation before another attribute macro runs.
#[forall(#[doc = "One state of the machine."])]
#[enum_fn(code: u8)]
pub enum State {
    Ready = 1,
    Busy = 2,
    Unknown,
}

/// Enum whose variants take their documentation before the discriminant macro runs.
#[forall(#[doc = "One kind of token."])]
#[discriminated_str(name)]
pub enum Kind {
    Ident = "ident",
    End = "end",
}

/// Declares one struct whose fields share a visibility and a type chosen by the caller.
macro_rules! shared_fields {
    ($(#[$attribute:meta])* $name:ident : $visibility:vis $shared:ty { $($field:ident),* $(,)? }) => {
        /// Struct generated with one shared field type.
        #[forall($(#[$attribute])* $visibility $shared)]
        pub struct $name($($field),*);
    };
}

shared_fields!(#[doc = "One forwarded field."] Forwarded: pub u16 { first, second });

/// Fields restricted to a module keep that visibility.
#[test]
fn distributes_a_restricted_visibility() {
    let scoped = restricted::scoped();

    assert_eq!(restricted::total(&scoped), 3);
}

/// Tuple positions spelled as raw identifiers name the fields.
#[test]
fn names_raw_identifier_fields() {
    let keywords = Keywords {
        r#type: 1,
        r#fn: 2,
        r#struct: 3,
    };

    assert_eq!(keywords.r#type + keywords.r#fn + keywords.r#struct, 6);
}

/// Lifetimes, const generics, defaults and bounds survive the renaming.
#[test]
fn shares_a_type_over_every_kind_of_generic() {
    let windows = Windows {
        current: &[1, 2],
        previous: &[3, 4],
    };

    assert_eq!(windows.current[1] + windows.previous[0], 5);
}

/// Trait objects and higher-ranked function pointers are shared like any type.
#[test]
fn shares_exotic_types() {
    let boxed = Boxed {
        twice: Box::new(|value| value * 2),
        thrice: Box::new(|value| value * 3),
    };
    let ranked = Ranked {
        identity: |value| value,
    };

    assert_eq!((boxed.twice)(2) + (boxed.thrice)(2), 10);
    assert_eq!(*(ranked.identity)(&7), 7);
}

/// A shared type may name the item it is distributed over.
#[test]
fn shares_a_recursive_type() {
    let node = Node {
        next: Some(Box::new(Node { next: None })),
    };

    assert!(node.next.is_some_and(|next| next.next.is_none()));
}

/// A shared type may be a macro call, expanded where the field is declared.
#[test]
fn shares_a_generated_type() {
    let produced = Produced {
        values: vec![1, 2],
        spare: Vec::new(),
    };

    assert_eq!(produced.values.len() + produced.spare.len(), 2);
}

/// Two prefixes compose, the outer one running before the inner.
#[test]
fn stacks_two_prefixes() {
    let stacked = Stacked {
        first: 1,
        second: 2,
    };

    assert_eq!(stacked.first + stacked.second, 3);
}

/// A distributed attribute reaches the derive macro that reads it.
#[test]
fn distributes_an_attribute_read_by_a_derive() {
    let defaults = Defaults { low: 1, high: 2 };

    assert_eq!(defaults.low + defaults.high, 3);
}

/// Another attribute macro runs on the item the prefix has already reached.
#[test]
fn composes_with_the_other_attribute_macros() {
    assert_eq!(State::Ready.code(), Some(1));
    assert_eq!(State::Busy.code(), Some(2));
    assert_eq!(State::Unknown.code(), None);
    assert_eq!(Kind::Ident.name(), "ident");
    assert_eq!(Kind::End.name(), "end");
}

/// A prefix forwarded by a declarative macro is read like one written directly.
#[test]
fn reads_a_forwarded_prefix() {
    let forwarded = Forwarded {
        first: 1,
        second: 2,
    };

    assert_eq!(forwarded.first + forwarded.second, 3);
}

/// Declares one struct whose fields share a type, with an optional forwarded visibility.
macro_rules! optional_visibility {
    ($name:ident : $visibility:vis $shared:ty { $($field:ident),* $(,)? }) => {
        /// Struct generated with an optional forwarded visibility.
        pub mod $name {
            //! Holds one struct whose forwarded prefix carries no visibility.

            /// Struct whose fields share a type and take no visibility.
            #[these_macros_should_be_illegal::forall($visibility $shared)]
            pub struct Fields($($field),*);

            /// Builds one value where its fields are visible.
            pub fn fields() -> Fields {
                Fields { only: 7 }
            }

            /// Reads the field the forwarded prefix left private.
            pub fn only(fields: &Fields) -> u8 {
                fields.only
            }
        }
    };
}

optional_visibility!(private_fields: u8 { only });

/// Declares one struct whose field names are forwarded as type fragments.
macro_rules! forwarded_names {
    ($name:ident : $shared:ty { $($position:ty),* $(,)? }) => {
        /// Struct generated from forwarded field names.
        #[forall(pub, $shared)]
        pub struct $name($($position),*);
    };
}

forwarded_names!(ForwardedNames: u8 { left, right });

/// A forwarded visibility that matched nothing leaves the fields private.
#[test]
fn reads_a_forwarded_prefix_without_a_visibility() {
    let fields = private_fields::fields();

    assert_eq!(private_fields::only(&fields), 7);
}

/// Field names forwarded as type fragments are read through their invisible groups.
#[test]
fn reads_forwarded_field_names() {
    let names = ForwardedNames { left: 1, right: 2 };

    assert_eq!(names.left + names.right, 3);
}
