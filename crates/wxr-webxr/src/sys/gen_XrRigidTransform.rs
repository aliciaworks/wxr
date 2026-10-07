#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRRigidTransform",
        typescript_type = "XRRigidTransform"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrRigidTransform` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRigidTransform)"]
    pub type XrRigidTransform;
    #[wasm_bindgen(method, getter, js_class = "XRRigidTransform", js_name = "position")]
    #[doc = "Getter for the `position` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRigidTransform/position)"]
    pub fn position(this: &XrRigidTransform) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, getter, js_class = "XRRigidTransform", js_name = "orientation")]
    #[doc = "Getter for the `orientation` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRigidTransform/orientation)"]
    pub fn orientation(this: &XrRigidTransform) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, getter, js_class = "XRRigidTransform", js_name = "matrix")]
    #[doc = "Getter for the `matrix` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRigidTransform/matrix)"]
    pub fn matrix(this: &XrRigidTransform) -> ::alloc::vec::Vec<f32>;
    #[wasm_bindgen(method, getter, js_class = "XRRigidTransform", js_name = "inverse")]
    #[doc = "Getter for the `inverse` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRigidTransform/inverse)"]
    pub fn inverse(this: &XrRigidTransform) -> XrRigidTransform;
    #[wasm_bindgen(catch, constructor, js_class = "XRRigidTransform")]
    #[doc = "The `new XrRigidTransform(..)` constructor, creating a new instance of `XrRigidTransform`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRigidTransform/XRRigidTransform)"]
    pub fn new() -> Result<XrRigidTransform, JsValue>;
}
