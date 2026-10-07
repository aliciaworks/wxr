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
        js_name = "XRJointSpace",
        typescript_type = "XRJointSpace"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrJointSpace` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRJointSpace)"]
    pub type XrJointSpace;
    #[wasm_bindgen(method, getter, js_class = "XRJointSpace", js_name = "jointName")]
    #[doc = "Getter for the `jointName` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRJointSpace/jointName)"]
    pub fn joint_name(this: &XrJointSpace) -> XrHandJoint;
}
