#![no_main]
use these_macros_should_be_illegal::enum_fn;
pub enum Action {
    Quit,
    Submit,
    InsertNewline,
}
#[automatically_derived]
impl ::core::clone::Clone for Action {
    #[inline]
    fn clone(&self) -> Action {
        *self
    }
}
#[automatically_derived]
impl ::core::marker::Copy for Action {}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Action {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Action {
    #[inline]
    fn eq(&self, other: &Action) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
    }
}
#[automatically_derived]
impl ::core::cmp::Eq for Action {
    #[inline]
    #[doc(hidden)]
    #[coverage(off)]
    fn assert_receiver_is_total_eq(&self) -> () {}
}
impl Action {
    #[allow(missing_docs)]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Quit => "quit spectral-m2",
            Self::Submit => "evaluate the input",
            Self::InsertNewline => "insert a line break",
            #[allow(unreachable_patterns)]
            _ => ::core::panicking::panic("explicit panic"),
        }
    }
}
pub enum Message {
    Tuple(u16),
    Struct { code: u8 },
    Dynamic(String),
}
impl Message {
    #[allow(missing_docs)]
    fn description<'__enum_fn_borrow>(
        &'__enum_fn_borrow self,
    ) -> &'__enum_fn_borrow str {
        match self {
            Self::Tuple(..) => "tuple payload",
            Self::Struct { .. } => "struct payload",
            Self::Dynamic(__enum_fn_field_0) => {
                (|value: &'__enum_fn_borrow String| value.as_str())(__enum_fn_field_0)
            }
            #[allow(unreachable_patterns)]
            _ => ::core::panicking::panic("explicit panic"),
        }
    }
}
pub enum Selected<'a> {
    Tuple(String, String),
    Struct { a: String, b: String },
    Borrowed(&'a str),
}
impl<'a> Selected<'a> {
    #[allow(missing_docs)]
    fn description<'__enum_fn_borrow>(
        &'__enum_fn_borrow self,
    ) -> &'__enum_fn_borrow str {
        match self {
            Self::Tuple(__enum_fn_field_0, __enum_fn_field_1) => {
                (|_: &'__enum_fn_borrow String, value: &'__enum_fn_borrow String| {
                    value.as_str()
                })(__enum_fn_field_0, __enum_fn_field_1)
            }
            Self::Struct { a: __enum_fn_field_0, b: __enum_fn_field_1 } => {
                (|a: &'__enum_fn_borrow String, b: &'__enum_fn_borrow String| {
                    let _ = b;
                    a.as_str()
                })(__enum_fn_field_0, __enum_fn_field_1)
            }
            Self::Borrowed(__enum_fn_field_0) => {
                (|value: &'__enum_fn_borrow &'a str| *value)(__enum_fn_field_0)
            }
            #[allow(unreachable_patterns)]
            _ => ::core::panicking::panic("explicit panic"),
        }
    }
}
pub enum Computed<'a> {
    Tuple(&'a str, &'a str),
    Named { code: u8, text: &'a str },
    Empty {},
    Unit,
}
impl<'a> Computed<'a> {
    #[allow(missing_docs)]
    fn description<'__enum_fn_borrow>(
        &'__enum_fn_borrow self,
    ) -> &'__enum_fn_borrow str {
        match self {
            Self::Tuple(__enum_fn_field_0, __enum_fn_field_1) => {
                (|_: &'__enum_fn_borrow &'a str, text: &'__enum_fn_borrow &'a str| {
                    *text
                })(__enum_fn_field_0, __enum_fn_field_1)
            }
            Self::Named { code: __enum_fn_field_0, text: __enum_fn_field_1 } => {
                (|code: &'__enum_fn_borrow u8, text: &'__enum_fn_borrow &'a str| {
                    let _ = code;
                    *text
                })(__enum_fn_field_0, __enum_fn_field_1)
            }
            Self::Empty {} => (|| "empty")(),
            Self::Unit => (|| "unit")(),
            #[allow(unreachable_patterns)]
            _ => ::core::panicking::panic("explicit panic"),
        }
    }
}
pub enum Optional<'a> {
    Fixed,
    Borrowed(&'a str),
    Missing,
    Payload(u8),
}
impl<'a> Optional<'a> {
    #[allow(missing_docs)]
    fn description<'__enum_fn_borrow>(
        &'__enum_fn_borrow self,
    ) -> Option<&'__enum_fn_borrow str> {
        match self {
            Self::Fixed => Some("fixed"),
            Self::Borrowed(__enum_fn_field_0) => {
                (|value: &'__enum_fn_borrow &'a str| Some(*value))(__enum_fn_field_0)
            }
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}
pub enum OptionalFixed {
    Fixed,
    Missing,
}
impl OptionalFixed {
    #[allow(missing_docs)]
    fn description(&self) -> Option<&'static str> {
        match self {
            Self::Fixed => Some("fixed"),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}
pub enum FixedFallback {
    Fixed,
    Missing,
    Payload(u8),
}
impl FixedFallback {
    #[allow(missing_docs)]
    pub(crate) fn description(&self) -> &'static str {
        match self {
            Self::Fixed => "fixed override",
            #[allow(unreachable_patterns)]
            _ => "fallback",
        }
    }
}
#[non_exhaustive]
pub enum OuterNonExhaustive {
    Known,
    Missing,
}
impl OuterNonExhaustive {
    #[allow(missing_docs)]
    fn code(&self) -> u8 {
        match self {
            Self::Known => 7,
            #[allow(unreachable_patterns)]
            _ => 255,
        }
    }
}
#[non_exhaustive]
pub enum InnerNonExhaustive {
    Known,
    Missing,
}
impl InnerNonExhaustive {
    #[allow(missing_docs)]
    fn code(&self) -> u8 {
        match self {
            Self::Known => 9,
            #[allow(unreachable_patterns)]
            _ => 255,
        }
    }
}
pub enum Panicking {
    Known,
}
impl Panicking {
    #[allow(missing_docs)]
    fn code(&self) -> u8 {
        match self {
            Self::Known => 3,
            #[allow(unreachable_patterns)]
            _ => {
                ::core::panicking::panic_fmt(format_args!("missing code"));
            }
        }
    }
}
