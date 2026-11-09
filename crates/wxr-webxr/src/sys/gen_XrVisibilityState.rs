#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrVisibilityState` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrVisibilityState {
    Visible = "visible",
    VisibleBlurred = "visible-blurred",
    Hidden = "hidden",
}
