#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrdomOverlayType` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrdomOverlayType {
    Screen = "screen",
    Floating = "floating",
    HeadLocked = "head-locked",
}
