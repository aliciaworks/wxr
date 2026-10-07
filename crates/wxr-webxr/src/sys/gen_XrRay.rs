#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRRay",
        typescript_type = "XRRay"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrRay` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRay)"]
    pub type XrRay;
    #[wasm_bindgen(method, getter, js_class = "XRRay", js_name = "origin")]
    #[doc = "Getter for the `origin` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRay/origin)"]
    pub fn origin(this: &XrRay) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, getter, js_class = "XRRay", js_name = "direction")]
    #[doc = "Getter for the `direction` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRay/direction)"]
    pub fn direction(this: &XrRay) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, getter, js_class = "XRRay", js_name = "matrix")]
    #[doc = "Getter for the `matrix` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRay/matrix)"]
    pub fn matrix(this: &XrRay) -> ::alloc::vec::Vec<f32>;
    #[wasm_bindgen(catch, constructor, js_class = "XRRay")]
    #[doc = "The `new XrRay(..)` constructor, creating a new instance of `XrRay`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRay/XRRay)"]
    pub fn new() -> Result<XrRay, JsValue>;
    #[wasm_bindgen(catch, constructor, js_class = "XRRay")]
    #[doc = "The `new XrRay(..)` constructor, creating a new instance of `XrRay`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRay/XRRay)"]
    pub fn new_with_transform(transform: &XrRigidTransform) -> Result<XrRay, JsValue>;
}
