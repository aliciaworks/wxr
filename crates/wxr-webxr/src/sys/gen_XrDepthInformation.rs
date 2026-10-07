#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRDepthInformation",
        typescript_type = "XRDepthInformation"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrDepthInformation` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRDepthInformation)"]
    pub type XrDepthInformation;
    #[wasm_bindgen(method, getter, js_class = "XRDepthInformation", js_name = "width")]
    #[doc = "Getter for the `width` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRDepthInformation/width)"]
    pub fn width(this: &XrDepthInformation) -> u32;
    #[wasm_bindgen(method, getter, js_class = "XRDepthInformation", js_name = "height")]
    #[doc = "Getter for the `height` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRDepthInformation/height)"]
    pub fn height(this: &XrDepthInformation) -> u32;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRDepthInformation",
        js_name = "normDepthBufferFromNormView"
    )]
    #[doc = "Getter for the `normDepthBufferFromNormView` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRDepthInformation/normDepthBufferFromNormView)"]
    pub fn norm_depth_buffer_from_norm_view(this: &XrDepthInformation) -> XrRigidTransform;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRDepthInformation",
        js_name = "rawValueToMeters"
    )]
    #[doc = "Getter for the `rawValueToMeters` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRDepthInformation/rawValueToMeters)"]
    pub fn raw_value_to_meters(this: &XrDepthInformation) -> f32;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRDepthInformation",
        js_name = "projectionMatrix"
    )]
    #[doc = "Getter for the `projectionMatrix` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRDepthInformation/projectionMatrix)"]
    pub fn projection_matrix(this: &XrDepthInformation) -> ::alloc::vec::Vec<f32>;
    #[wasm_bindgen(method, getter, js_class = "XRDepthInformation", js_name = "transform")]
    #[doc = "Getter for the `transform` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRDepthInformation/transform)"]
    pub fn transform(this: &XrDepthInformation) -> XrRigidTransform;
}
