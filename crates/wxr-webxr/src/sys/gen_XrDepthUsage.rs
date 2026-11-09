#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrDepthUsage` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrDepthUsage {
    CpuOptimized = "cpu-optimized",
    GpuOptimized = "gpu-optimized",
}
