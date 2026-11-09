#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRTransientInputHitTestResult",
        typescript_type = "XRTransientInputHitTestResult"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrTransientInputHitTestResult` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRTransientInputHitTestResult)"]
    pub type XrTransientInputHitTestResult;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRTransientInputHitTestResult",
        js_name = "inputSource"
    )]
    #[doc = "Getter for the `inputSource` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRTransientInputHitTestResult/inputSource)"]
    pub fn input_source(this: &XrTransientInputHitTestResult) -> XrInputSource;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRTransientInputHitTestResult",
        js_name = "results"
    )]
    #[doc = "Getter for the `results` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRTransientInputHitTestResult/results)"]
    pub fn results(this: &XrTransientInputHitTestResult) -> ::js_sys::Array;
}
