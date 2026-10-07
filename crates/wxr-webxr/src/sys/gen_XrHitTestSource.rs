#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRHitTestSource",
        typescript_type = "XRHitTestSource"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrHitTestSource` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRHitTestSource)"]
    pub type XrHitTestSource;
    #[wasm_bindgen(method, js_class = "XRHitTestSource")]
    #[doc = "The `cancel()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRHitTestSource/cancel)"]
    pub fn cancel(this: &XrHitTestSource);
}
