#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRHitTestResult",
        typescript_type = "XRHitTestResult"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrHitTestResult` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRHitTestResult)"]
    pub type XrHitTestResult;
    #[wasm_bindgen(method, js_class = "XRHitTestResult", js_name = "getPose")]
    #[doc = "The `getPose()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRHitTestResult/getPose)"]
    pub fn get_pose(this: &XrHitTestResult, base_space: &XrSpace) -> Option<XrPose>;
}
