#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrDepthDataFormat` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrDepthDataFormat {
    LuminanceAlpha = "luminance-alpha",
    Float32 = "float32",
    UnsignedShort = "unsigned-short",
}
