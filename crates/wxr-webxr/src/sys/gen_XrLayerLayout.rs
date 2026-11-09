#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrLayerLayout` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrLayerLayout {
    Default = "default",
    Mono = "mono",
    Stereo = "stereo",
    StereoLeftRight = "stereo-left-right",
    StereoTopBottom = "stereo-top-bottom",
}
