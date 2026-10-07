#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRTransientInputHitTestSource",
        typescript_type = "XRTransientInputHitTestSource"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrTransientInputHitTestSource` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRTransientInputHitTestSource)"]
    pub type XrTransientInputHitTestSource;
    #[wasm_bindgen(method, js_class = "XRTransientInputHitTestSource")]
    #[doc = "The `cancel()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRTransientInputHitTestSource/cancel)"]
    pub fn cancel(this: &XrTransientInputHitTestSource);
}
