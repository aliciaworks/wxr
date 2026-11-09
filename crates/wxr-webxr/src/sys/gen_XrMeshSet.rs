#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRMeshSet",
        typescript_type = "XRMeshSet"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrMeshSet` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMeshSet)"]
    pub type XrMeshSet;
    #[wasm_bindgen(method, getter, js_class = "XRMeshSet", js_name = "size")]
    #[doc = "Getter for the `size` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMeshSet/size)"]
    pub fn size(this: &XrMeshSet) -> u32;
    #[wasm_bindgen(catch, method, js_class = "XRMeshSet", js_name = "forEach")]
    #[doc = "The `forEach()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMeshSet/forEach)"]
    pub fn for_each(this: &XrMeshSet, callback: &::js_sys::Function) -> Result<(), JsValue>;
    #[wasm_bindgen(method, js_class = "XRMeshSet")]
    #[doc = "The `has()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMeshSet/has)"]
    pub fn has(this: &XrMeshSet, value: &XrMesh) -> bool;
    #[wasm_bindgen(method, js_class = "XRMeshSet")]
    #[doc = "The `entries()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMeshSet/entries)"]
    pub fn entries(this: &XrMeshSet) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRMeshSet")]
    #[doc = "The `keys()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMeshSet/keys)"]
    pub fn keys(this: &XrMeshSet) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRMeshSet")]
    #[doc = "The `values()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMeshSet/values)"]
    pub fn values(this: &XrMeshSet) -> ::js_sys::Iterator;
}
