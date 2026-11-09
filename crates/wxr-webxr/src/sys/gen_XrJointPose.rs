#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrPose",
        extends = "::js_sys::Object",
        js_name = "XRJointPose",
        typescript_type = "XRJointPose"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrJointPose` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRJointPose)"]
    pub type XrJointPose;
    #[wasm_bindgen(method, getter, js_class = "XRJointPose", js_name = "radius")]
    #[doc = "Getter for the `radius` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRJointPose/radius)"]
    pub fn radius(this: &XrJointPose) -> f32;
}
