//! String literals derived from Rust names and types.

use heck::{ToKebabCase, ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
    Error, LitStr, Token, Type,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse2,
    spanned::Spanned,
};

use crate::helpers::arguments::reject_macro_call;

/// Spelling options accepted before the `;` of `stringify_as!`, other than `type`.
const CASE_OPTIONS: [(&str, Case); 7] = [
    ("camel", Case::Camel),
    ("pascal", Case::Pascal),
    ("snake", Case::Snake),
    ("kebab", Case::Kebab),
    ("screaming_snake", Case::ScreamingSnake),
    ("lower", Case::Lower),
    ("upper", Case::Upper),
];

/// Spelling option selecting the canonical type name.
const TYPE_OPTION: &str = "type";

/// Supported conventional spellings of a name.
#[derive(Clone, Copy)]
enum Case {
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
    /// Unicode lowercase while retaining existing separators.
    Lower,
    /// Unicode uppercase while retaining existing separators.
    Upper,
}

impl Case {
    /// Spells `name` in this case.
    fn convert(self, name: &str) -> String {
        match self {
            Self::Camel => name.to_lower_camel_case(),
            Self::Pascal => name.to_upper_camel_case(),
            Self::Snake => name.to_snake_case(),
            Self::Kebab => name.to_kebab_case(),
            Self::ScreamingSnake => name.to_shouty_snake_case(),
            Self::Lower => name.to_lowercase(),
            Self::Upper => name.to_uppercase(),
        }
    }
}

/// How `stringify_as!` spells its input.
#[derive(Clone, Copy)]
enum Spelling {
    /// One name converted to a conventional case.
    Case(Case),
    /// One type in canonical `type:` form.
    Type,
}

impl Parse for Spelling {
    /// Parses one option name from [`CASE_OPTIONS`] or [`TYPE_OPTION`].
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let option = Ident::parse_any(input)?;
        if option == TYPE_OPTION {
            return Ok(Self::Type);
        }
        CASE_OPTIONS
            .iter()
            .find(|(name, _)| option == name)
            .map(|&(_, case)| Self::Case(case))
            .ok_or_else(|| {
                let names: Vec<_> = CASE_OPTIONS.iter().map(|(name, _)| *name).collect();
                Error::new(
                    option.span(),
                    format!("expected one of: {}, {TYPE_OPTION}", names.join(", ")),
                )
            })
    }
}

/// Complete `stringify_as!` input: a spelling option, `;`, and the tokens to spell.
struct Invocation {
    /// Selected spelling.
    spelling: Spelling,
    /// Name or type to spell.
    body: TokenStream,
}

impl Parse for Invocation {
    /// Parses `option; body` without imposing a grammar on the body yet.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let spelling = input.parse()?;
        input.parse::<Token![;]>()?;
        Ok(Self {
            spelling,
            body: input.parse()?,
        })
    }
}

macro_docs! {
    /// Spells one name in a conventional case, or one type canonically, as a string literal.
    ///
    /// The option before `;` selects the spelling: `camel`, `pascal`, `snake`, `kebab`,
    /// `screaming_snake`, `lower` or `upper` convert an identifier or string literal,
    /// respecting acronym boundaries; `lower` and `upper` retain existing separators.
    /// `type` produces a canonical, collision-safe name for one Rust type: it begins
    /// with `type:`, which is deliberately illegal in Rust identifiers, and drops
    /// formatting while preserving structure, so `& 'a mut Vec < Option < T > >`
    /// becomes `"type:&'a mut Vec<Option<T>>"`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use these_macros_should_be_illegal::stringify_as;
    /// assert_eq!(stringify_as!(camel; some_HTTP_server), "someHttpServer");
    /// assert_eq!(stringify_as!(screaming_snake; SomeHTTPServer), "SOME_HTTP_SERVER");
    /// assert_eq!(
    ///     stringify_as!(type; fn((u8, u16), *const [u8; 4]) -> bool),
    ///     "type:fn((u8,u16),*const[u8;4])->bool",
    /// );
    /// ```
}

/// Spells a name in the selected case or a type canonically: `stringify_as!(snake; Name)`.
pub fn stringify_as(input: TokenStream) -> TokenStream {
    match parse2::<Invocation>(input) {
        Ok(Invocation {
            spelling: Spelling::Case(case),
            body,
        }) => stringify_case(body, case),
        Ok(Invocation {
            spelling: Spelling::Type,
            body,
        }) => stringify_type(body),
        Err(error) => error.into_compile_error(),
    }
}

/// Produces a compact, collision-safe name for a parsed Rust type.
///
/// The `type:` prefix is deliberately illegal in Rust identifiers, while the
/// remainder retains the type's structural punctuation.
fn normalize_type(ty: &Type) -> String {
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
fn stringify_case(input: TokenStream, case: Case) -> TokenStream {
    parse_name(input)
        .map(|name| {
            let value = LitStr::new(&case.convert(&name.value), name.span);
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
fn stringify_type(input: TokenStream) -> TokenStream {
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

impl Parse for Name {
    /// Parses exactly one supported source spelling.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        reject_macro_call(input, "an identifier or string literal")?;
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
            // An invisible group only marks a forwarded fragment; its contents are the type.
            TokenTree::Group(group) if group.delimiter() == Delimiter::None => {
                previous_word = write_stream(group.stream(), output, previous_word);
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
        Delimiter::None => ("", ""),
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

    use super::{Case, normalize_type, stringify_as, stringify_case, stringify_type};

    /// Every case converts names independently of macro parsing.
    #[test]
    fn converts_names_with_cases() {
        assert_eq!(Case::Camel.convert("some_HTTP_server"), "someHttpServer");
        assert_eq!(Case::Pascal.convert("some_HTTP_server"), "SomeHttpServer");
        assert_eq!(Case::Snake.convert("SomeHTTPServer"), "some_http_server");
        assert_eq!(Case::Kebab.convert("SomeHTTPServer"), "some-http-server");
        assert_eq!(Case::ScreamingSnake.convert("SomeHTTPServer"), "SOME_HTTP_SERVER");
        assert_eq!(Case::Lower.convert("Some_Name"), "some_name");
        assert_eq!(Case::Upper.convert("Some_Name"), "SOME_NAME");
    }

    /// The leading option selects the spelling, and unknown options are rejected.
    #[test]
    fn selects_spelling_by_option() {
        assert_eq!(
            stringify_as(quote!(kebab; SomeHTTPServer)).to_string(),
            r#""some-http-server""#
        );
        assert_eq!(
            stringify_as(quote!(type; Vec<T>)).to_string(),
            r#""type:Vec<T>""#
        );
        assert!(
            stringify_as(quote!(title; name))
                .to_string()
                .contains("expected one of")
        );
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

    /// A type forwarded through `macro_rules!` is named exactly as if written directly.
    #[test]
    fn names_forwarded_types_like_written_ones() {
        let forwarded = proc_macro2::Group::new(proc_macro2::Delimiter::None, quote!(Vec<u8>));
        assert_eq!(
            stringify_type(quote!(#forwarded)).to_string(),
            stringify_type(quote!(Vec<u8>)).to_string()
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
