#![no_main]

use these_macros_should_be_illegal::forall;

#[forall(pub(crate))]
pub struct Pixel(u8, u8, u8);

#[forall(pub, f32)]
pub struct Point(x, y);

#[forall(u8, pub, #[doc = "One channel of the shared triple."])]
pub struct Channels(#[doc = "Intensity of the red channel."] red, green, blue);

#[forall(pub, Vec<T>)]
pub struct Buckets<T>(small, large)
where
    T: Clone;

#[forall(#[doc = "One shared field."] pub)]
pub struct Window {
    width: u32,
    pub(crate) height: u32,
}

#[forall(u16, String)]
pub enum Response {
    Redirect,
    Error(&'static str),
}

#[forall({ code: u16, path: String })]
pub enum Request {
    Read,
    Write { payload: u8 },
}

#[forall(#[doc = "One stage of the pipeline."])]
pub enum Stage {
    Parse,
    Lower { depth: usize },
}

#[forall(#[doc = "One shared field."] pub)]
pub struct Partial {
    width: u32,
    #[forall(skip)]
    height: u32,
    #[forall(skip(pub))]
    depth: u32,
}

#[forall(u16, String)]
pub enum Reply {
    Moved,
    #[forall(skip(String))]
    Empty,
}
