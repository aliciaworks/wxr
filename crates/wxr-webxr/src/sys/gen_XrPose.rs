#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRPose",
        typescript_type = "XRPose"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrPose` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPose)"]
    pub type XrPose;
    #[wasm_bindgen(method, getter, js_class = "XRPose", js_name = "transform")]
    #[doc = "Getter for the `transform` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPose/transform)"]
    pub fn transform(this: &XrPose) -> XrRigidTransform;
    #[wasm_bindgen(method, getter, js_class = "XRPose", js_name = "linearVelocity")]
    #[doc = "Getter for the `linearVelocity` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPose/linearVelocity)"]
    pub fn linear_velocity(this: &XrPose) -> Option<DomPointReadOnly>;
    #[wasm_bindgen(method, getter, js_class = "XRPose", js_name = "angularVelocity")]
    #[doc = "Getter for the `angularVelocity` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPose/angularVelocity)"]
    pub fn angular_velocity(this: &XrPose) -> Option<DomPointReadOnly>;
    #[wasm_bindgen(method, getter, js_class = "XRPose", js_name = "emulatedPosition")]
    #[doc = "Getter for the `emulatedPosition` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPose/emulatedPosition)"]
    pub fn emulated_position(this: &XrPose) -> bool;
}
