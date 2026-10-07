#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrCompositionLayer",
        extends = "XrLayer",
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "XRProjectionLayer",
        typescript_type = "XRProjectionLayer"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrProjectionLayer` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRProjectionLayer)"]
    pub type XrProjectionLayer;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRProjectionLayer",
        js_name = "textureWidth"
    )]
    #[doc = "Getter for the `textureWidth` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRProjectionLayer/textureWidth)"]
    pub fn texture_width(this: &XrProjectionLayer) -> u32;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRProjectionLayer",
        js_name = "textureHeight"
    )]
    #[doc = "Getter for the `textureHeight` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRProjectionLayer/textureHeight)"]
    pub fn texture_height(this: &XrProjectionLayer) -> u32;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRProjectionLayer",
        js_name = "textureArrayLength"
    )]
    #[doc = "Getter for the `textureArrayLength` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRProjectionLayer/textureArrayLength)"]
    pub fn texture_array_length(this: &XrProjectionLayer) -> u32;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRProjectionLayer",
        js_name = "ignoreDepthValues"
    )]
    #[doc = "Getter for the `ignoreDepthValues` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRProjectionLayer/ignoreDepthValues)"]
    pub fn ignore_depth_values(this: &XrProjectionLayer) -> bool;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRProjectionLayer",
        js_name = "fixedFoveation"
    )]
    #[doc = "Getter for the `fixedFoveation` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRProjectionLayer/fixedFoveation)"]
    pub fn fixed_foveation(this: &XrProjectionLayer) -> Option<f32>;
    #[wasm_bindgen(
        method,
        setter,
        js_class = "XRProjectionLayer",
        js_name = "fixedFoveation"
    )]
    #[doc = "Setter for the `fixedFoveation` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRProjectionLayer/fixedFoveation)"]
    pub fn set_fixed_foveation(this: &XrProjectionLayer, value: Option<f32>);
    #[wasm_bindgen(method, getter, js_class = "XRProjectionLayer", js_name = "deltaPose")]
    #[doc = "Getter for the `deltaPose` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRProjectionLayer/deltaPose)"]
    pub fn delta_pose(this: &XrProjectionLayer) -> Option<XrRigidTransform>;
    #[wasm_bindgen(method, setter, js_class = "XRProjectionLayer", js_name = "deltaPose")]
    #[doc = "Setter for the `deltaPose` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRProjectionLayer/deltaPose)"]
    pub fn set_delta_pose(this: &XrProjectionLayer, value: Option<&XrRigidTransform>);
}
