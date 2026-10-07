#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "Gamepad",
        typescript_type = "Gamepad"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `Gamepad` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Gamepad)"]
    pub type Gamepad;
}
