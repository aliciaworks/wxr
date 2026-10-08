#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRImageTrackingResult",
        typescript_type = "XRImageTrackingResult"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrImageTrackingResult` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRImageTrackingResult)"]
    pub type XrImageTrackingResult;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRImageTrackingResult",
        js_name = "imageSpace"
    )]
    #[doc = "Getter for the `imageSpace` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRImageTrackingResult/imageSpace)"]
    pub fn image_space(this: &XrImageTrackingResult) -> XrSpace;
    #[wasm_bindgen(method, getter, js_class = "XRImageTrackingResult", js_name = "index")]
    #[doc = "Getter for the `index` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRImageTrackingResult/index)"]
    pub fn index(this: &XrImageTrackingResult) -> u32;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRImageTrackingResult",
        js_name = "trackingState"
    )]
    #[doc = "Getter for the `trackingState` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRImageTrackingResult/trackingState)"]
    pub fn tracking_state(this: &XrImageTrackingResult) -> XrImageTrackingState;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRImageTrackingResult",
        js_name = "measuredWidthInMeters"
    )]
    #[doc = "Getter for the `measuredWidthInMeters` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRImageTrackingResult/measuredWidthInMeters)"]
    pub fn measured_width_in_meters(this: &XrImageTrackingResult) -> f32;
}
