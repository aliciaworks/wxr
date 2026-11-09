#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrSpace",
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "XRBodySpace",
        typescript_type = "XRBodySpace"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrBodySpace` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRBodySpace)"]
    pub type XrBodySpace;
    #[wasm_bindgen(method, getter, js_class = "XRBodySpace", js_name = "jointName")]
    #[doc = "Getter for the `jointName` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRBodySpace/jointName)"]
    pub fn joint_name(this: &XrBodySpace) -> XrBodyJoint;
}
