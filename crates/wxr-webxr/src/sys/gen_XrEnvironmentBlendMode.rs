#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrEnvironmentBlendMode` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrEnvironmentBlendMode {
    Opaque = "opaque",
    AlphaBlend = "alpha-blend",
    Additive = "additive",
}
