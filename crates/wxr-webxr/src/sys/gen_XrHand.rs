#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRHand",
        typescript_type = "XRHand"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrHand` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRHand)"]
    pub type XrHand;
    #[wasm_bindgen(method, getter, js_class = "XRHand", js_name = "size")]
    #[doc = "Getter for the `size` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRHand/size)"]
    pub fn size(this: &XrHand) -> u32;
    #[wasm_bindgen(catch, method, js_class = "XRHand", js_name = "forEach")]
    #[doc = "The `forEach()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRHand/forEach)"]
    pub fn for_each(this: &XrHand, callback: &::js_sys::Function) -> Result<(), JsValue>;
    #[wasm_bindgen(method, js_class = "XRHand")]
    #[doc = "The `get()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRHand/get)"]
    pub fn get(this: &XrHand, key: XrHandJoint) -> XrJointSpace;
    #[wasm_bindgen(method, js_class = "XRHand")]
    #[doc = "The `entries()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRHand/entries)"]
    pub fn entries(this: &XrHand) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRHand")]
    #[doc = "The `keys()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRHand/keys)"]
    pub fn keys(this: &XrHand) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRHand")]
    #[doc = "The `values()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRHand/values)"]
    pub fn values(this: &XrHand) -> ::js_sys::Iterator;
}
