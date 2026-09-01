//! String literals derived from Rust names and types.

use heck::{ToKebabCase, ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{Error, LitStr, Type, ext::IdentExt, parse::ParseStream, parse2, spanned::Spanned};

/// Supported conventional spellings of a name.
#[derive(Clone, Copy)]
pub(crate) enum Case {
    /// `camelCase`, more precisely lower camel case.
    Camel,
    /// `PascalCase`, also called upper camel case.
    Pascal,
    /// `snake_case`.
    Snake,
    /// `kebab-case`.
    Kebab,
    /// `SCREAMING_SNAKE_CASE`.
    ScreamingSnake,
    /// `lowercase` without word separators.
    Lower,
    /// `UPPERCASE` without word separators.
    Upper,
}

/// Converts a name to `camelCase` (lower camel case).
pub(crate) fn camel_case(name: &str) -> String {
    name.to_lower_camel_case()
}

/// Converts a name to `PascalCase` (upper camel case).
pub(crate) fn pascal_case(name: &str) -> String {
    name.to_upper_camel_case()
}

/// Converts a name to `snake_case`.
pub(crate) fn snake_case(name: &str) -> String {
    name.to_snake_case()
}

/// Converts a name to `kebab-case`.
pub(crate) fn kebab_case(name: &str) -> String {
    name.to_kebab_case()
}

/// Converts a name to `SCREAMING_SNAKE_CASE`.
pub(crate) fn screaming_snake_case(name: &str) -> String {
    name.to_shouty_snake_case()
}

/// Lowercases a name without otherwise changing its separators.
pub(crate) fn lowercase(name: &str) -> String {
    name.to_lowercase()
}

/// Uppercases a name without otherwise changing its separators.
pub(crate) fn uppercase(name: &str) -> String {
    name.to_uppercase()
}

/// Produces a compact, collision-safe name for a parsed Rust type.
///
/// The `type:` prefix is deliberately illegal in Rust identifiers, while the
/// remainder retains the type's structural punctuation.
pub(crate) fn normalize_type(ty: &Type) -> String {
    let mut tokens = TokenStream::new();
    ty.to_tokens(&mut tokens);
    format!("type:{}", compact(tokens))
}

/// One identifier or string literal carrying the source name.
struct Name {
    /// Name spelling without raw-identifier syntax or string delimiters.
    value: String,
    /// Location used for the resulting literal and any diagnostic.
    span: Span,
}

/// Converts exactly one identifier or string literal to the requested case.
pub(crate) fn stringify_case(input: TokenStream, case: Case) -> TokenStream {
    parse_name(input)
        .map(|name| {
            let value = match case {
                Case::Camel => camel_case(&name.value),
                Case::Pascal => pascal_case(&name.value),
                Case::Snake => snake_case(&name.value),
                Case::Kebab => kebab_case(&name.value),
                Case::ScreamingSnake => screaming_snake_case(&name.value),
                Case::Lower => lowercase(&name.value),
                Case::Upper => uppercase(&name.value),
            };
            let value = LitStr::new(&value, name.span);
            quote!(#value)
        })
        .unwrap_or_else(Error::into_compile_error)
}

/// Produces a compact canonical spelling for exactly one Rust type.
///
/// The `type:` prefix makes even a simple result such as `type:String`
/// impossible to confuse with a legal Rust identifier. Compound types retain
/// Rust's own structural punctuation, for example
/// `type:&'a mut Vec<Option<T>>`.
pub(crate) fn stringify_type(input: TokenStream) -> TokenStream {
    parse2::<Type>(input)
        .map(|ty| {
            let value = LitStr::new(&normalize_type(&ty), ty.span());
            quote!(#value)
        })
        .unwrap_or_else(Error::into_compile_error)
}

/// Parses one identifier, including keywords and raw identifiers, or a string.
fn parse_name(input: TokenStream) -> syn::Result<Name> {
    parse2(input).and_then(|input: Name| {
        if input.value.is_empty() {
            Err(Error::new(input.span, "expected a non-empty name"))
        } else {
            Ok(input)
        }
    })
}

impl syn::parse::Parse for Name {
    /// Parses exactly one supported source spelling.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = if input.peek(LitStr) {
            let literal: LitStr = input.parse()?;
            Self {
                value: literal.value(),
                span: literal.span(),
            }
        } else {
            let ident = Ident::parse_any(input)?;
            Self {
                value: ident.unraw().to_string(),
                span: ident.span(),
            }
        };

        if !input.is_empty() {
            return Err(input.error("expected exactly one identifier or string literal"));
        }
        Ok(name)
    }
}

/// Serializes a parsed type without formatting-dependent whitespace.
fn compact(tokens: TokenStream) -> String {
    let mut output = String::new();
    write_stream(tokens, &mut output, false);
    output
}

/// Appends one stream, separating adjacent word-like tokens when necessary.
fn write_stream(tokens: TokenStream, output: &mut String, mut previous_word: bool) -> bool {
    for token in tokens {
        match token {
            TokenTree::Ident(ident) => {
                if previous_word {
                    output.push(' ');
                }
                output.push_str(&ident.to_string());
                previous_word = true;
            }
            TokenTree::Literal(literal) => {
                if previous_word {
                    output.push(' ');
                }
                output.push_str(&literal.to_string());
                previous_word = true;
            }
            TokenTree::Punct(punct) => {
                if punct.as_char() == '\'' && previous_word {
                    output.push(' ');
                }
                output.push(punct.as_char());
                previous_word = false;
            }
            TokenTree::Group(group) => {
                previous_word = write_group(group, output);
            }
        }
    }
    previous_word
}

/// Appends one delimited token group.
fn write_group(group: Group, output: &mut String) -> bool {
    let (open, close) = match group.delimiter() {
        Delimiter::Parenthesis => ("(", ")"),
        Delimiter::Brace => ("{", "}"),
        Delimiter::Bracket => ("[", "]"),
        Delimiter::None => ("$none(", ")"),
    };
    output.push_str(open);
    write_stream(group.stream(), output, false);
    output.push_str(close);
    false
}

/// Focused conversion and canonicalization tests.
#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::Type;

    use super::{
        Case, camel_case, kebab_case, lowercase, normalize_type, pascal_case, screaming_snake_case,
        snake_case, stringify_case, stringify_type, uppercase,
    };

    /// The reusable functions expose every conversion independently of macros.
    #[test]
    fn converts_names_with_functions() {
        assert_eq!(camel_case("some_HTTP_server"), "someHttpServer");
        assert_eq!(pascal_case("some_HTTP_server"), "SomeHttpServer");
        assert_eq!(snake_case("SomeHTTPServer"), "some_http_server");
        assert_eq!(kebab_case("SomeHTTPServer"), "some-http-server");
        assert_eq!(screaming_snake_case("SomeHTTPServer"), "SOME_HTTP_SERVER");
        assert_eq!(lowercase("Some_Name"), "some_name");
        assert_eq!(uppercase("Some_Name"), "SOME_NAME");
    }

    /// Parsed types can be normalized without passing through a proc macro.
    #[test]
    fn normalizes_types_with_a_function() {
        let ty: Type = syn::parse_quote!(&'a mut Vec<Option<T>>);
        assert_eq!(normalize_type(&ty), "type:&'a mut Vec<Option<T>>");
    }

    /// Case conversion respects acronym boundaries and separator-based input.
    #[test]
    fn converts_name_cases() {
        assert_eq!(
            stringify_case(quote!(HTTPServer), Case::Snake).to_string(),
            r#""http_server""#
        );
        assert_eq!(
            stringify_case(quote!(snake_case), Case::Pascal).to_string(),
            r#""SnakeCase""#
        );
        assert_eq!(
            stringify_case(quote!("kebab-case"), Case::Camel).to_string(),
            r#""kebabCase""#
        );
    }

    /// Raw identifiers contribute their actual identifier rather than `r#`.
    #[test]
    fn converts_raw_identifiers() {
        assert_eq!(
            stringify_case(quote!(r#type), Case::Upper).to_string(),
            r#""TYPE""#
        );
    }

    /// The canonical type name retains all structural type distinctions.
    #[test]
    fn normalizes_compound_types() {
        assert_eq!(
            stringify_type(quote!(&'a mut Vec<Option<T>>)).to_string(),
            r#""type:&'a mut Vec<Option<T>>""#
        );
        assert_eq!(
            stringify_type(quote!(*const [fn(u8) -> bool; 3])).to_string(),
            r#""type:*const[fn(u8)->bool;3]""#
        );
    }
}
