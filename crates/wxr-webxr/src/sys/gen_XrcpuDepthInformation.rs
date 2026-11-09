#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrDepthInformation",
        extends = "::js_sys::Object",
        js_name = "XRCPUDepthInformation",
        typescript_type = "XRCPUDepthInformation"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrcpuDepthInformation` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCPUDepthInformation)"]
    pub type XrcpuDepthInformation;
    #[wasm_bindgen(method, getter, js_class = "XRCPUDepthInformation", js_name = "data")]
    #[doc = "Getter for the `data` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCPUDepthInformation/data)"]
    pub fn data(this: &XrcpuDepthInformation) -> ::js_sys::ArrayBuffer;
    #[wasm_bindgen(
        method,
        js_class = "XRCPUDepthInformation",
        js_name = "getDepthInMeters"
    )]
    #[doc = "The `getDepthInMeters()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCPUDepthInformation/getDepthInMeters)"]
    pub fn get_depth_in_meters(this: &XrcpuDepthInformation, x: f32, y: f32) -> f32;
}
