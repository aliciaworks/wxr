#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRSubImage",
        typescript_type = "XRSubImage"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrSubImage` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSubImage)"]
    pub type XrSubImage;
    #[wasm_bindgen(method, getter, js_class = "XRSubImage", js_name = "viewport")]
    #[doc = "Getter for the `viewport` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSubImage/viewport)"]
    pub fn viewport(this: &XrSubImage) -> XrViewport;
}
