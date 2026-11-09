#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRInputSourceArray",
        typescript_type = "XRInputSourceArray"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrInputSourceArray` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourceArray)"]
    pub type XrInputSourceArray;
    #[wasm_bindgen(method, getter, js_class = "XRInputSourceArray", js_name = "length")]
    #[doc = "Getter for the `length` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourceArray/length)"]
    pub fn length(this: &XrInputSourceArray) -> u32;
    #[wasm_bindgen(catch, method, js_class = "XRInputSourceArray", js_name = "forEach")]
    #[doc = "The `forEach()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourceArray/forEach)"]
    pub fn for_each(
        this: &XrInputSourceArray,
        callback: &::js_sys::Function,
    ) -> Result<(), JsValue>;
    #[wasm_bindgen(method, js_class = "XRInputSourceArray", indexing_getter)]
    #[doc = "Indexing getter. As in the literal Javascript `this[key]`."]
    #[doc = ""]
    pub fn get(this: &XrInputSourceArray, index: u32) -> Option<XrInputSource>;
    #[wasm_bindgen(method, js_class = "XRInputSourceArray")]
    #[doc = "The `entries()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourceArray/entries)"]
    pub fn entries(this: &XrInputSourceArray) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRInputSourceArray")]
    #[doc = "The `keys()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourceArray/keys)"]
    pub fn keys(this: &XrInputSourceArray) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRInputSourceArray")]
    #[doc = "The `values()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourceArray/values)"]
    pub fn values(this: &XrInputSourceArray) -> ::js_sys::Iterator;
}
