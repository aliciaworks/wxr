#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrLayerQuality` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrLayerQuality {
    Default = "default",
    TextOptimized = "text-optimized",
    GraphicsOptimized = "graphics-optimized",
}
