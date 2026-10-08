#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRExpressions",
        typescript_type = "XRExpressions"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrExpressions` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRExpressions)"]
    pub type XrExpressions;
    #[wasm_bindgen(method, getter, js_class = "XRExpressions", js_name = "size")]
    #[doc = "Getter for the `size` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRExpressions/size)"]
    pub fn size(this: &XrExpressions) -> u32;
    #[wasm_bindgen(catch, method, js_class = "XRExpressions", js_name = "forEach")]
    #[doc = "The `forEach()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRExpressions/forEach)"]
    pub fn for_each(this: &XrExpressions, callback: &::js_sys::Function) -> Result<(), JsValue>;
    #[wasm_bindgen(method, js_class = "XRExpressions")]
    #[doc = "The `get()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRExpressions/get)"]
    pub fn get(this: &XrExpressions, key: XrExpression) -> f32;
    #[wasm_bindgen(method, js_class = "XRExpressions")]
    #[doc = "The `entries()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRExpressions/entries)"]
    pub fn entries(this: &XrExpressions) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRExpressions")]
    #[doc = "The `keys()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRExpressions/keys)"]
    pub fn keys(this: &XrExpressions) -> ::js_sys::Iterator;
    #[wasm_bindgen(method, js_class = "XRExpressions")]
    #[doc = "The `values()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRExpressions/values)"]
    pub fn values(this: &XrExpressions) -> ::js_sys::Iterator;
}
