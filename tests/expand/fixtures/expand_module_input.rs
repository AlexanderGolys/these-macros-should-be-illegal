pub fn owned_string() -> String {
    match @@ "s" {
        a@@@"s" if &a == "s" => {
            @@ "ok"
        },
        _ => { a + &@@"better not land here" }
    }
}
