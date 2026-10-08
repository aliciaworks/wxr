#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRBody",
        typescript_type = "XRBody"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrBody` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRBody)"]
    pub type XrBody;
    #[wasm_bindgen(method, getter, js_class = "XRBody", js_name = "size")]
    #[doc = "Getter for the `size` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRBody/size)"]
    pub fn size(this: &XrBody) -> u32;
    #[wasm_bindgen(catch, method, js_class = "XRBody", js_name = "forEach")]
    #[doc = "The `forEach()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRBody/forEach)"]
    pub fn for_each(this: &XrBody, callback: &::js_sys::Function) -> Result<(), JsValue>;
    #[wasm_bindgen(method, js_class = "XRBody")]
    #[doc = "The `get()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRBody/get)"]
    pub fn get(this: &XrBody, key: XrBodyJoint) -> XrBodySpace;
    #[wasm_bindgen(method, js_class = "XRBody")]
    #[doc = "The `entries()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRBody/entries)"]
    pub fn entries(this: &XrBody) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRBody")]
    #[doc = "The `keys()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRBody/keys)"]
    pub fn keys(this: &XrBody) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRBody")]
    #[doc = "The `values()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRBody/values)"]
    pub fn values(this: &XrBody) -> ::js_sys::Iterator;
}
