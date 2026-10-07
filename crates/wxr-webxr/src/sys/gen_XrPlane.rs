#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRPlane",
        typescript_type = "XRPlane"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrPlane` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPlane)"]
    pub type XrPlane;
    #[wasm_bindgen(method, getter, js_class = "XRPlane", js_name = "planeSpace")]
    #[doc = "Getter for the `planeSpace` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPlane/planeSpace)"]
    pub fn plane_space(this: &XrPlane) -> XrSpace;
    #[wasm_bindgen(method, getter, js_class = "XRPlane", js_name = "polygon")]
    #[doc = "Getter for the `polygon` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPlane/polygon)"]
    pub fn polygon(this: &XrPlane) -> ::js_sys::Array;
    #[wasm_bindgen(method, getter, js_class = "XRPlane", js_name = "orientation")]
    #[doc = "Getter for the `orientation` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPlane/orientation)"]
    pub fn orientation(this: &XrPlane) -> Option<XrPlaneOrientation>;
    #[wasm_bindgen(method, getter, js_class = "XRPlane", js_name = "semanticLabel")]
    #[doc = "Getter for the `semanticLabel` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPlane/semanticLabel)"]
    pub fn semantic_label(this: &XrPlane) -> Option<::alloc::string::String>;
}
