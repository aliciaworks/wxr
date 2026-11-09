#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "XRSpace",
        typescript_type = "XRSpace"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrSpace` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSpace)"]
    pub type XrSpace;
}
