#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRAnchor",
        typescript_type = "XRAnchor"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrAnchor` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRAnchor)"]
    pub type XrAnchor;
    #[wasm_bindgen(method, getter, js_class = "XRAnchor", js_name = "anchorSpace")]
    #[doc = "Getter for the `anchorSpace` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRAnchor/anchorSpace)"]
    pub fn anchor_space(this: &XrAnchor) -> XrSpace;
    #[wasm_bindgen(method, js_class = "XRAnchor")]
    #[doc = "The `delete()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRAnchor/delete)"]
    pub fn delete(this: &XrAnchor);
    #[wasm_bindgen(method, js_class = "XRAnchor", js_name = "requestPersistentHandle")]
    #[doc = "The `requestPersistentHandle()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRAnchor/requestPersistentHandle)"]
    pub fn request_persistent_handle(this: &XrAnchor) -> ::js_sys::Promise;
}
