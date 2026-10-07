#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrReferenceSpaceType` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrReferenceSpaceType {
    Viewer = "viewer",
    Local = "local",
    LocalFloor = "local-floor",
    BoundedFloor = "bounded-floor",
    Unbounded = "unbounded",
}
