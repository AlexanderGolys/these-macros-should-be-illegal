//! Consumer tests for compile-time name and type stringification.

use these_macros_should_be_illegal::{
    stringify_camel_case, stringify_kebab_case, stringify_lowercase, stringify_pascal_case,
    stringify_screaming_snake_case, stringify_snake_case, stringify_type, stringify_uppercase,
};

const CAMEL: &str = stringify_camel_case!(some_HTTP_server);
const PASCAL: &str = stringify_pascal_case!(some_HTTP_server);
const SNAKE: &str = stringify_snake_case!(SomeHTTPServer);
const KEBAB: &str = stringify_kebab_case!(SomeHTTPServer);
const SCREAMING_SNAKE: &str = stringify_screaming_snake_case!(SomeHTTPServer);
const LOWER: &str = stringify_lowercase!(Some_HTTP_Server);
const UPPER: &str = stringify_uppercase!(Some_HTTP_Server);
const FROM_KEBAB: &str = stringify_pascal_case!("some-http-server");

/// Every name conversion expands directly to a string literal.
#[test]
fn converts_names_in_both_directions() {
    assert_eq!(CAMEL, "someHttpServer");
    assert_eq!(PASCAL, "SomeHttpServer");
    assert_eq!(SNAKE, "some_http_server");
    assert_eq!(KEBAB, "some-http-server");
    assert_eq!(SCREAMING_SNAKE, "SOME_HTTP_SERVER");
    assert_eq!(LOWER, "some_http_server");
    assert_eq!(UPPER, "SOME_HTTP_SERVER");
    assert_eq!(FROM_KEBAB, "SomeHttpServer");
}

/// Type normalization is compact, stable across layout, and never an identifier.
#[test]
fn normalizes_rust_types() {
    const SIMPLE: &str = stringify_type!(String);
    const BORROWED: &str = stringify_type!(&'static mut Vec<Option<String>>);
    const FUNCTION: &str = stringify_type!(fn((u8, u16), *const [u8; 4]) -> bool);

    assert_eq!(SIMPLE, "type:String");
    assert_eq!(BORROWED, "type:&'static mut Vec<Option<String>>");
    assert_eq!(FUNCTION, "type:fn((u8,u16),*const[u8;4])->bool");
}
