//! Helper for external-module loading and function-like macro injection.

use std::{fs, path::PathBuf};

use proc_macro2::{Delimiter, LexError, Spacing, TokenStream, TokenTree, fallback};
use quote::quote;
use syn::{
    Error, Expr, ExprLit, Ident, ItemMod, Lit, Meta, Path, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse2,
};

use TokenTree::{Group as GroupTT, Ident as IdentTT, Punct as PunctTT};

use super::preprocessing::{ExpansionConfig, parse_config_option, split_config_prefix};

/// Macro paths and shared options supplied before the module declaration.
struct Arguments {
    /// Function-like macros to wrap around the loaded module body.
    macros: Vec<Path>,
    /// Shared preprocessing configuration passed to each injected macro.
    config: ExpansionConfig,
}

/// Complete input to the external-module expander.
struct Invocation {
    /// Macro injection arguments before the separating semicolon.
    arguments: Arguments,
    /// Out-of-line module declaration whose source will be loaded.
    module: ItemMod,
}

/// Parses the external-module expander's leading arguments.
impl Parse for Arguments {
    /// Parses macro paths interspersed with supported named configuration options.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut macros = Vec::new();
        let mut config = ExpansionConfig::default();
        let mut has_excluded_macros = false;

        while !input.is_empty() {
            let fork = input.fork();
            let is_option = Ident::parse_any(&fork).is_ok() && fork.peek(Token![=]);

            if is_option {
                parse_config_option(input, &mut config, &mut has_excluded_macros)?;
            } else {
                macros.push(input.parse()?);
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        if macros.is_empty() {
            return Err(input.error("expected at least one function-like macro"));
        }

        Ok(Self { macros, config })
    }
}

/// Parses the external-module expander's complete invocation.
impl Parse for Invocation {
    /// Parses arguments, their separator, and one out-of-line module declaration.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut argument_tokens = TokenStream::new();

        while !input.peek(Token![;]) {
            if input.is_empty() {
                return Err(input.error("expected `;` before the module declaration"));
            }
            argument_tokens.extend([input.parse::<TokenTree>()?]);
        }
        input.parse::<Token![;]>()?;

        let arguments = parse2(argument_tokens)?;
        let module = input.parse()?;

        if !input.is_empty() {
            return Err(input.error("unexpected tokens after the module declaration"));
        }

        Ok(Self { arguments, module })
    }
}

/// Loads an out-of-line module and injects the requested function-like macros.
pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let invocation_file = input
        .clone()
        .into_iter()
        .next()
        .and_then(|token| token.span().local_file());

    inject(input, invocation_file).unwrap_or_else(Error::into_compile_error)
}

/// Loads a module body and nests it in the configured function-like macro invocations.
fn inject(input: TokenStream, invocation_file: Option<PathBuf>) -> syn::Result<TokenStream> {
    let (config, input) = split_config_prefix(input)?;
    let mut invocation = parse2::<Invocation>(input)?;
    invocation.arguments.config.merge(config);
    let Invocation { arguments, module } = invocation;

    if module.content.is_some() {
        return Err(Error::new_spanned(
            &module,
            "`expand` expects an out-of-line module declaration",
        ));
    }

    let invocation_file = invocation_file.ok_or_else(|| {
        Error::new_spanned(&module.ident, "cannot determine the invoking source file")
    })?;
    let module_file = module_file(&module, invocation_file)?;
    let source = fs::read_to_string(&module_file).map_err(|error| {
        Error::new_spanned(
            &module.ident,
            format!("failed to read `{}`: {error}", module_file.display()),
        )
    })?;
    let body = lex_source(&source).map_err(|error| {
        Error::new_spanned(
            &module.ident,
            format!("failed to lex `{}`: {error}", module_file.display()),
        )
    })?;
    let mut body: TokenStream = fallback_to_compiler(body)?.into();

    for macro_path in arguments.macros.iter().rev() {
        let input = arguments.config.configure_input(body);
        body = quote!(#macro_path! { #input });
    }

    let attrs: Vec<_> = module
        .attrs
        .iter()
        .filter(|attribute| !attribute.path().is_ident("path"))
        .collect();
    let vis = &module.vis;
    let unsafety = &module.unsafety;
    let ident = &module.ident;

    Ok(quote! {
        #(#attrs)*
        #vis #unsafety mod #ident {
            #body
        }
    })
}

/// Lexes external source with proc-macro2's fallback implementation enabled.
fn lex_source(source: &str) -> Result<TokenStream, LexError> {
    /// Restores compiler-backed tokenization after fallback lexing leaves scope.
    struct FallbackLexer;

    /// Restores proc-macro2's normal lexer selection at scope exit.
    impl Drop for FallbackLexer {
        /// Disables the forced fallback lexer.
        fn drop(&mut self) {
            fallback::unforce();
        }
    }

    fallback::force();
    let fallback = FallbackLexer;
    let tokens = source.parse()?;
    drop(fallback);

    Ok(tokens)
}

/// Converts a fallback proc-macro2 stream into compiler-owned procedural-macro tokens.
fn fallback_to_compiler(input: TokenStream) -> syn::Result<proc_macro::TokenStream> {
    input.into_iter().map(fallback_token_to_compiler).collect()
}

/// Converts one fallback token and recursively converts group contents.
fn fallback_token_to_compiler(token: TokenTree) -> syn::Result<proc_macro::TokenTree> {
    let span = proc_macro::Span::call_site();

    Ok(match token {
        GroupTT(group) => {
            let delimiter = match group.delimiter() {
                Delimiter::Parenthesis => proc_macro::Delimiter::Parenthesis,
                Delimiter::Brace => proc_macro::Delimiter::Brace,
                Delimiter::Bracket => proc_macro::Delimiter::Bracket,
                Delimiter::None => proc_macro::Delimiter::None,
            };
            let mut converted =
                proc_macro::Group::new(delimiter, fallback_to_compiler(group.stream())?);
            converted.set_span(span);
            proc_macro::TokenTree::Group(converted)
        }
        PunctTT(punctuation) => {
            let spacing = match punctuation.spacing() {
                Spacing::Alone => proc_macro::Spacing::Alone,
                Spacing::Joint => proc_macro::Spacing::Joint,
            };
            let mut converted = proc_macro::Punct::new(punctuation.as_char(), spacing);
            converted.set_span(span);
            proc_macro::TokenTree::Punct(converted)
        }
        IdentTT(identifier) => compiler_token(&identifier.to_string())?,
        TokenTree::Literal(literal) => compiler_token(&literal.to_string())?,
    })
}

/// Re-parses one identifier or literal spelling as exactly one compiler token.
fn compiler_token(spelling: &str) -> syn::Result<proc_macro::TokenTree> {
    let tokens = spelling
        .parse::<proc_macro::TokenStream>()
        .map_err(|error| {
            Error::new(
                proc_macro2::Span::call_site(),
                format!("failed to convert token `{spelling}`: {error}"),
            )
        })?;
    let mut tokens = tokens.into_iter();
    let token = tokens
        .next()
        .ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "token conversion was empty"))?;

    if tokens.next().is_some() {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            format!("token conversion produced multiple tokens for `{spelling}`"),
        ));
    }

    Ok(token)
}

/// Resolves an out-of-line module declaration to its source file.
fn module_file(module: &ItemMod, invocation_file: PathBuf) -> syn::Result<PathBuf> {
    let parent = invocation_file.parent().ok_or_else(|| {
        Error::new_spanned(
            &module.ident,
            "the invoking source file has no parent directory",
        )
    })?;

    if let Some(path) = explicit_module_path(module)? {
        return Ok(parent.join(path));
    }

    let module_directory = match invocation_file.file_name().and_then(|name| name.to_str()) {
        Some("lib.rs" | "main.rs" | "mod.rs") => parent.to_owned(),
        _ => parent.join(invocation_file.file_stem().ok_or_else(|| {
            Error::new_spanned(&module.ident, "the invoking source file has no file stem")
        })?),
    };
    let name = module.ident.unraw().to_string();
    let flat = module_directory.join(format!("{name}.rs"));
    let nested = module_directory.join(&name).join("mod.rs");

    match (flat.exists(), nested.exists()) {
        (true, false) => Ok(flat),
        (false, true) => Ok(nested),
        (true, true) => Err(Error::new_spanned(
            &module.ident,
            format!(
                "module source exists at both `{}` and `{}`",
                flat.display(),
                nested.display()
            ),
        )),
        (false, false) => Err(Error::new_spanned(
            &module.ident,
            format!(
                "module source was not found at `{}` or `{}`",
                flat.display(),
                nested.display()
            ),
        )),
    }
}

/// Reads and validates a module declaration's optional explicit `path` attribute.
fn explicit_module_path(module: &ItemMod) -> syn::Result<Option<PathBuf>> {
    let Some(attribute) = module
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("path"))
    else {
        return Ok(None);
    };
    let Meta::NameValue(meta) = &attribute.meta else {
        return Err(Error::new_spanned(
            attribute,
            "expected `#[path = \"...\"]`",
        ));
    };
    let Expr::Lit(ExprLit {
        lit: Lit::Str(path),
        ..
    }) = &meta.value
    else {
        return Err(Error::new_spanned(
            &meta.value,
            "module path must be a string literal",
        ));
    };

    Ok(Some(path.value().into()))
}

#[cfg(test)]
mod tests {
    //! Unit tests for external-module invocation parsing.

    use super::*;

    /// Parses macro paths and shared options before loading source.
    #[test]
    fn parses_macros_and_shared_expansion_config() {
        let arguments: Arguments = syn::parse_str(
            "literally_literal_string, ::other::syntax, exclude_macros = (raw_tokens,)",
        )
        .unwrap();

        assert_eq!(arguments.macros.len(), 2);
        assert!(arguments.config.is_excluded(&syn::parse_quote!(raw_tokens)));
    }

    /// Parses the complete invocation without attempting file-system access.
    #[test]
    fn parses_expander_invocation_before_module_source_is_loaded() {
        let invocation: Invocation =
            syn::parse_str("literally_literal_string; #[path = \"custom.rs\"] pub mod custom;")
                .unwrap();

        assert_eq!(invocation.arguments.macros.len(), 1);
        assert_eq!(invocation.module.ident, "custom");
        assert!(invocation.module.content.is_none());
    }
}
