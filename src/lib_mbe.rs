//! Declarative helpers that keep the crate root free of entry-point boilerplate.

/// Declares the public documentation of a module's macro, written as doc comments.
///
/// Expands to a module-local `docs!()` returning the comments as one string, which
/// `macro_cat_mod!` attaches to the exported entry point.
macro_rules! macro_docs {
    ($(#[doc = $line:literal])*) => {
        /// Public documentation of this module's macro, as one string.
        macro_rules! docs {
            () => { concat!($($line, "\n"),*) };
        }
        pub(crate) use docs;
    };
}

/// Declares one category module of function-like macros and exports their entry points.
///
/// Written like the module it declares, with the macro in place of `mod`:
/// `macro_cat_mod! { name //! docs  entries }`. Each entry `name;` declares the module
/// `category/name.rs` and exports a macro delegating to its same-name function,
/// documented by its `macro_docs!`; `name [alias, ...];` exports one identical macro
/// per alias.
macro_rules! macro_cat_mod {
    (@export proc_macro $category:ident::$module:ident as $name:ident) => {
        #[doc = $category::$module::docs!()]
        #[proc_macro]
        pub fn $name(input: TokenStream) -> TokenStream {
            $category::$module::$module(input.into()).into()
        }
    };
    (@export proc_macro_attribute $category:ident::$module:ident as $name:ident) => {
        #[doc = $category::$module::docs!()]
        #[proc_macro_attribute]
        pub fn $name(arguments: TokenStream, item: TokenStream) -> TokenStream {
            $category::$module::$module(arguments.into(), item.into()).into()
        }
    };
    (
        @$kind:ident $category:ident $(#![$attr:meta])*
        $($name:ident $([$($alias:ident),* $(,)?])?;)*
    ) => {
        mod $category {
            $(#![$attr])*
            $(pub mod $name;)*
        }
        $(
            macro_cat_mod!(@export $kind $category::$name as $name);
            $($(macro_cat_mod!(@export $kind $category::$name as $alias);)*)?
        )*
    };
    ($category:ident $($body:tt)*) => {
        macro_cat_mod!(@proc_macro $category $($body)*);
    };
}

/// Declares one category module of attribute macros, with the syntax of `macro_cat_mod!`.
macro_rules! attr_macro_cat_mod {
    ($category:ident $($body:tt)*) => {
        macro_cat_mod!(@proc_macro_attribute $category $($body)*);
    };
}
