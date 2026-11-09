#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrDepthInformation",
        extends = "::js_sys::Object",
        js_name = "XRWebGLDepthInformation",
        typescript_type = "XRWebGLDepthInformation"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrWebGlDepthInformation` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLDepthInformation)"]
    pub type XrWebGlDepthInformation;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLDepthInformation",
        js_name = "texture"
    )]
    #[doc = "Getter for the `texture` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLDepthInformation/texture)"]
    pub fn texture(this: &XrWebGlDepthInformation) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLDepthInformation",
        js_name = "textureType"
    )]
    #[doc = "Getter for the `textureType` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLDepthInformation/textureType)"]
    pub fn texture_type(this: &XrWebGlDepthInformation) -> XrTextureType;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLDepthInformation",
        js_name = "imageIndex"
    )]
    #[doc = "Getter for the `imageIndex` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLDepthInformation/imageIndex)"]
    pub fn image_index(this: &XrWebGlDepthInformation) -> Option<u32>;
}
