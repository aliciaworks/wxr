#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrHitTestTrackableType` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrHitTestTrackableType {
    Point = "point",
    Plane = "plane",
    Mesh = "mesh",
}
