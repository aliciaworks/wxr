#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrReferenceSpace",
        extends = "XrSpace",
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "XRBoundedReferenceSpace",
        typescript_type = "XRBoundedReferenceSpace"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrBoundedReferenceSpace` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRBoundedReferenceSpace)"]
    pub type XrBoundedReferenceSpace;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRBoundedReferenceSpace",
        js_name = "boundsGeometry"
    )]
    #[doc = "Getter for the `boundsGeometry` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRBoundedReferenceSpace/boundsGeometry)"]
    pub fn bounds_geometry(this: &XrBoundedReferenceSpace) -> ::js_sys::Array;
}
