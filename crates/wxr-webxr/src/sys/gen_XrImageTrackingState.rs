#![allow(unused_imports)]
#![allow(clippy::all)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[doc = "The `XrImageTrackingState` enum."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrImageTrackingState {
    Untracked = "untracked",
    Tracked = "tracked",
    Emulated = "emulated",
}
