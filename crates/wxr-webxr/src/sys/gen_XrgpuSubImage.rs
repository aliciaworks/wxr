#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrSubImage",
        extends = "::js_sys::Object",
        js_name = "XRGPUSubImage",
        typescript_type = "XRGPUSubImage"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrgpuSubImage` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUSubImage)"]
    pub type XrgpuSubImage;
    #[wasm_bindgen(method, getter, js_class = "XRGPUSubImage", js_name = "colorTexture")]
    #[doc = "Getter for the `colorTexture` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUSubImage/colorTexture)"]
    pub fn color_texture(this: &XrgpuSubImage) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRGPUSubImage",
        js_name = "depthStencilTexture"
    )]
    #[doc = "Getter for the `depthStencilTexture` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUSubImage/depthStencilTexture)"]
    pub fn depth_stencil_texture(this: &XrgpuSubImage) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRGPUSubImage",
        js_name = "motionVectorTexture"
    )]
    #[doc = "Getter for the `motionVectorTexture` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUSubImage/motionVectorTexture)"]
    pub fn motion_vector_texture(this: &XrgpuSubImage) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, js_class = "XRGPUSubImage", js_name = "getViewDescriptor")]
    #[doc = "The `getViewDescriptor()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRGPUSubImage/getViewDescriptor)"]
    pub fn get_view_descriptor(this: &XrgpuSubImage) -> ::wasm_bindgen::JsValue;
}
