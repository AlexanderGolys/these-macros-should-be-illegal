macro_rules! raw_tokens {
    ($($tokens:tt)*) => {
        "borrowed"
    };
}

pub fn owned_string() -> String {
    let value = @@"loaded outside Rust";
    value
}

#[allow(clippy::let_and_return)]
pub fn ordinary_rust_passes_through() -> &'static str {
    let value = "still borrowed";
    value
}

pub fn excluded_macro_arguments_are_opaque() -> &'static str {
    raw_tokens!(@@"untouched")
}
