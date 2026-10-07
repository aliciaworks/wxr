#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRAnchorSet",
        typescript_type = "XRAnchorSet"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrAnchorSet` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRAnchorSet)"]
    pub type XrAnchorSet;
    #[wasm_bindgen(method, getter, js_class = "XRAnchorSet", js_name = "size")]
    #[doc = "Getter for the `size` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRAnchorSet/size)"]
    pub fn size(this: &XrAnchorSet) -> u32;
    #[wasm_bindgen(catch, method, js_class = "XRAnchorSet", js_name = "forEach")]
    #[doc = "The `forEach()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRAnchorSet/forEach)"]
    pub fn for_each(this: &XrAnchorSet, callback: &::js_sys::Function) -> Result<(), JsValue>;
    #[wasm_bindgen(method, js_class = "XRAnchorSet")]
    #[doc = "The `has()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRAnchorSet/has)"]
    pub fn has(this: &XrAnchorSet, value: &XrAnchor) -> bool;
    #[wasm_bindgen(method, js_class = "XRAnchorSet")]
    #[doc = "The `entries()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRAnchorSet/entries)"]
    pub fn entries(this: &XrAnchorSet) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRAnchorSet")]
    #[doc = "The `keys()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRAnchorSet/keys)"]
    pub fn keys(this: &XrAnchorSet) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRAnchorSet")]
    #[doc = "The `values()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRAnchorSet/values)"]
    pub fn values(this: &XrAnchorSet) -> ::js_sys::Iterator;
}
