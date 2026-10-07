#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrInteractionMode` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrInteractionMode {
    ScreenSpace = "screen-space",
    WorldSpace = "world-space",
}
