#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrDepthInformation",
        extends = "::js_sys::Object",
        js_name = "XRGPUDepthInformation",
        typescript_type = "XRGPUDepthInformation"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrgpuDepthInformation` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUDepthInformation)"]
    pub type XrgpuDepthInformation;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRGPUDepthInformation",
        js_name = "texture"
    )]
    #[doc = "Getter for the `texture` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUDepthInformation/texture)"]
    pub fn texture(this: &XrgpuDepthInformation) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(
        method,
        js_class = "XRGPUDepthInformation",
        js_name = "getViewDescriptor"
    )]
    #[doc = "The `getViewDescriptor()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUDepthInformation/getViewDescriptor)"]
    pub fn get_view_descriptor(this: &XrgpuDepthInformation) -> ::wasm_bindgen::JsValue;
}
