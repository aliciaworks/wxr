#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrTargetRayMode` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrTargetRayMode {
    Gaze = "gaze",
    TrackedPointer = "tracked-pointer",
    Screen = "screen",
    TransientPointer = "transient-pointer",
}
