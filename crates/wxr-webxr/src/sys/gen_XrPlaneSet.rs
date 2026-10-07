#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRPlaneSet",
        typescript_type = "XRPlaneSet"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrPlaneSet` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPlaneSet)"]
    pub type XrPlaneSet;
    #[wasm_bindgen(method, getter, js_class = "XRPlaneSet", js_name = "size")]
    #[doc = "Getter for the `size` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPlaneSet/size)"]
    pub fn size(this: &XrPlaneSet) -> u32;
    #[wasm_bindgen(catch, method, js_class = "XRPlaneSet", js_name = "forEach")]
    #[doc = "The `forEach()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPlaneSet/forEach)"]
    pub fn for_each(this: &XrPlaneSet, callback: &::js_sys::Function) -> Result<(), JsValue>;
    #[wasm_bindgen(method, js_class = "XRPlaneSet")]
    #[doc = "The `has()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPlaneSet/has)"]
    pub fn has(this: &XrPlaneSet, value: &XrPlane) -> bool;
    #[wasm_bindgen(method, js_class = "XRPlaneSet")]
    #[doc = "The `entries()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPlaneSet/entries)"]
    pub fn entries(this: &XrPlaneSet) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRPlaneSet")]
    #[doc = "The `keys()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPlaneSet/keys)"]
    pub fn keys(this: &XrPlaneSet) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRPlaneSet")]
    #[doc = "The `values()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPlaneSet/values)"]
    pub fn values(this: &XrPlaneSet) -> ::js_sys::Iterator;
}
