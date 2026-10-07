#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrSessionMode` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrSessionMode {
    Inline = "inline",
    ImmersiveVr = "immersive-vr",
    ImmersiveAr = "immersive-ar",
}
