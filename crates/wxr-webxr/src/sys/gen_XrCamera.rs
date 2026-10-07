#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRCamera",
        typescript_type = "XRCamera"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrCamera` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCamera)"]
    pub type XrCamera;
    #[wasm_bindgen(method, getter, js_class = "XRCamera", js_name = "width")]
    #[doc = "Getter for the `width` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCamera/width)"]
    pub fn width(this: &XrCamera) -> u32;
    #[wasm_bindgen(method, getter, js_class = "XRCamera", js_name = "height")]
    #[doc = "Getter for the `height` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCamera/height)"]
    pub fn height(this: &XrCamera) -> u32;
}
