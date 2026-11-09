#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRViewport",
        typescript_type = "XRViewport"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrViewport` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRViewport)"]
    pub type XrViewport;
    #[wasm_bindgen(method, getter, js_class = "XRViewport", js_name = "x")]
    #[doc = "Getter for the `x` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRViewport/x)"]
    pub fn x(this: &XrViewport) -> i32;
    #[wasm_bindgen(method, getter, js_class = "XRViewport", js_name = "y")]
    #[doc = "Getter for the `y` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRViewport/y)"]
    pub fn y(this: &XrViewport) -> i32;
    #[wasm_bindgen(method, getter, js_class = "XRViewport", js_name = "width")]
    #[doc = "Getter for the `width` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRViewport/width)"]
    pub fn width(this: &XrViewport) -> i32;
    #[wasm_bindgen(method, getter, js_class = "XRViewport", js_name = "height")]
    #[doc = "Getter for the `height` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRViewport/height)"]
    pub fn height(this: &XrViewport) -> i32;
}
