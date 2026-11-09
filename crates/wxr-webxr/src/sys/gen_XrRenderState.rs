#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRRenderState",
        typescript_type = "XRRenderState"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrRenderState` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRenderState)"]
    pub type XrRenderState;
    #[wasm_bindgen(method, getter, js_class = "XRRenderState", js_name = "depthNear")]
    #[doc = "Getter for the `depthNear` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRenderState/depthNear)"]
    pub fn depth_near(this: &XrRenderState) -> f64;
    #[wasm_bindgen(method, getter, js_class = "XRRenderState", js_name = "depthFar")]
    #[doc = "Getter for the `depthFar` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRenderState/depthFar)"]
    pub fn depth_far(this: &XrRenderState) -> f64;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRRenderState",
        js_name = "passthroughFullyObscured"
    )]
    #[doc = "Getter for the `passthroughFullyObscured` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRenderState/passthroughFullyObscured)"]
    pub fn passthrough_fully_obscured(this: &XrRenderState) -> Option<bool>;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRRenderState",
        js_name = "inlineVerticalFieldOfView"
    )]
    #[doc = "Getter for the `inlineVerticalFieldOfView` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRenderState/inlineVerticalFieldOfView)"]
    pub fn inline_vertical_field_of_view(this: &XrRenderState) -> Option<f64>;
    #[wasm_bindgen(method, getter, js_class = "XRRenderState", js_name = "baseLayer")]
    #[doc = "Getter for the `baseLayer` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRenderState/baseLayer)"]
    pub fn base_layer(this: &XrRenderState) -> Option<XrWebGlLayer>;
    #[wasm_bindgen(method, getter, js_class = "XRRenderState", js_name = "layers")]
    #[doc = "Getter for the `layers` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRRenderState/layers)"]
    pub fn layers(this: &XrRenderState) -> ::js_sys::Array;
}
