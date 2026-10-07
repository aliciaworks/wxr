#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRView",
        typescript_type = "XRView"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrView` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRView)"]
    pub type XrView;
    #[wasm_bindgen(method, getter, js_class = "XRView", js_name = "isFirstPersonObserver")]
    #[doc = "Getter for the `isFirstPersonObserver` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRView/isFirstPersonObserver)"]
    pub fn is_first_person_observer(this: &XrView) -> bool;
    #[wasm_bindgen(method, getter, js_class = "XRView", js_name = "eye")]
    #[doc = "Getter for the `eye` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRView/eye)"]
    pub fn eye(this: &XrView) -> XrEye;
    #[wasm_bindgen(method, getter, js_class = "XRView", js_name = "index")]
    #[doc = "Getter for the `index` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRView/index)"]
    pub fn index(this: &XrView) -> u32;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRView",
        js_name = "recommendedViewportScale"
    )]
    #[doc = "Getter for the `recommendedViewportScale` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRView/recommendedViewportScale)"]
    pub fn recommended_viewport_scale(this: &XrView) -> Option<f64>;
    #[wasm_bindgen(method, getter, js_class = "XRView", js_name = "projectionMatrix")]
    #[doc = "Getter for the `projectionMatrix` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRView/projectionMatrix)"]
    pub fn projection_matrix(this: &XrView) -> ::alloc::vec::Vec<f32>;
    #[wasm_bindgen(method, getter, js_class = "XRView", js_name = "transform")]
    #[doc = "Getter for the `transform` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRView/transform)"]
    pub fn transform(this: &XrView) -> XrRigidTransform;
    #[wasm_bindgen(method, js_class = "XRView", js_name = "requestViewportScale")]
    #[doc = "The `requestViewportScale()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRView/requestViewportScale)"]
    pub fn request_viewport_scale(this: &XrView, scale: Option<f64>);
}
