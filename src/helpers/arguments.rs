//! Argument grammars shared by attribute macros.
//!
//! Each grammar accepts one optional trailing comma; anything left over is
//! rejected by `syn::parse2` itself.

use syn::{
    Ident, Path, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};

/// Rejects a macro call where a name or literal must be read directly.
///
/// Macro arguments reach this crate unexpanded, so a macro call can only stand
/// where its expansion is emitted unchanged, never where its value is inspected.
pub(crate) fn reject_macro_call(input: ParseStream, expected: &str) -> syn::Result<()> {
    let fork = input.fork();
    if fork.call(Path::parse_mod_style).is_ok() && fork.peek(Token![!]) {
        return Err(input.error(format!(
            "expected {expected}, found a macro call; macro calls in these arguments are not expanded"
        )));
    }
    Ok(())
}

/// Empty argument list of an attribute whose item already says everything.
pub(crate) struct NoArguments;

impl Parse for NoArguments {
    /// Accepts only an empty argument list.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            Ok(Self)
        } else {
            Err(input.error("this attribute takes no arguments"))
        }
    }
}

/// Name of a method, keywords and raw identifiers included.
pub(crate) struct MethodName(pub(crate) Ident);

impl Parse for MethodName {
    /// Parses one identifier and an optional trailing comma.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        reject_macro_call(input, "a method name")?;
        let name = Ident::parse_any(input)?;
        input.parse::<Option<Token![,]>>()?;
        Ok(Self(name))
    }
}

/// Shared argument grammar tests.
#[cfg(test)]
mod tests {
    use syn::parse_str;

    use super::{MethodName, NoArguments};

    /// A method name accepts keywords, raw identifiers and one trailing comma.
    #[test]
    fn parses_method_names() {
        assert_eq!(parse_str::<MethodName>("apply,").unwrap().0, "apply");
        assert_eq!(parse_str::<MethodName>("type").unwrap().0, "type");
        assert_eq!(parse_str::<MethodName>("r#type").unwrap().0, "r#type");
        assert!(parse_str::<MethodName>("apply, other").is_err());
    }

    /// A macro call standing for a name is rejected with an explanation.
    #[test]
    fn rejects_macro_calls_as_names() {
        let error = parse_str::<MethodName>("name!()").err().unwrap().to_string();
        assert!(error.contains("found a macro call"), "{error}");
    }

    /// Only an empty argument list is accepted where none are expected.
    #[test]
    fn rejects_any_arguments() {
        assert!(parse_str::<NoArguments>("").is_ok());
        assert!(parse_str::<NoArguments>("anything").is_err());
    }
}
