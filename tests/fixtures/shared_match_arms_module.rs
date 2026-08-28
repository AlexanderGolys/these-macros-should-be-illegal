/// Values whose payloads stringify through distinct concrete methods.
enum Value {
    /// Numeric value.
    Number(u32),
    /// Character value.
    Character(char),
}

/// Describes either payload through one source-level RHS.
fn describe(value: Value) -> String {
    match value {
        Value::Number(value) || Value::Character(value) => value.to_string(),
    }
}

/// Describes one numeric payload.
pub fn describe_number(value: u32) -> String {
    describe(Value::Number(value))
}

/// Describes one character payload.
pub fn describe_character(value: char) -> String {
    describe(Value::Character(value))
}
