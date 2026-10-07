#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "Navigator",
        typescript_type = "Navigator"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `Navigator` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator)"]
    pub type Navigator;
    #[wasm_bindgen(method, getter, js_class = "Navigator", js_name = "xr")]
    #[doc = "Getter for the `xr` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/xr)"]
    pub fn xr(this: &Navigator) -> XrSystem;
}
