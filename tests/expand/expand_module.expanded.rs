#![no_main]
mod expanded {
    pub fn owned_string() -> String {
        match ::std::string::String::from("s") {
            a@::std::string::String::from("s") if &a == "s" => {
                ::std::string::String::from("ok")
            },
            _ => { a + &::std::string::String::from("better not land here") }
        }

    }
}
