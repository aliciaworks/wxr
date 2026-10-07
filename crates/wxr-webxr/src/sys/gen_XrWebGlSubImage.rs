#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrSubImage",
        extends = "::js_sys::Object",
        js_name = "XRWebGLSubImage",
        typescript_type = "XRWebGLSubImage"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrWebGlSubImage` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLSubImage)"]
    pub type XrWebGlSubImage;
    #[wasm_bindgen(method, getter, js_class = "XRWebGLSubImage", js_name = "colorTexture")]
    #[doc = "Getter for the `colorTexture` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLSubImage/colorTexture)"]
    pub fn color_texture(this: &XrWebGlSubImage) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLSubImage",
        js_name = "depthStencilTexture"
    )]
    #[doc = "Getter for the `depthStencilTexture` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLSubImage/depthStencilTexture)"]
    pub fn depth_stencil_texture(this: &XrWebGlSubImage) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLSubImage",
        js_name = "motionVectorTexture"
    )]
    #[doc = "Getter for the `motionVectorTexture` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLSubImage/motionVectorTexture)"]
    pub fn motion_vector_texture(this: &XrWebGlSubImage) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, getter, js_class = "XRWebGLSubImage", js_name = "imageIndex")]
    #[doc = "Getter for the `imageIndex` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLSubImage/imageIndex)"]
    pub fn image_index(this: &XrWebGlSubImage) -> Option<u32>;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLSubImage",
        js_name = "colorTextureWidth"
    )]
    #[doc = "Getter for the `colorTextureWidth` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLSubImage/colorTextureWidth)"]
    pub fn color_texture_width(this: &XrWebGlSubImage) -> u32;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLSubImage",
        js_name = "colorTextureHeight"
    )]
    #[doc = "Getter for the `colorTextureHeight` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLSubImage/colorTextureHeight)"]
    pub fn color_texture_height(this: &XrWebGlSubImage) -> u32;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLSubImage",
        js_name = "depthStencilTextureWidth"
    )]
    #[doc = "Getter for the `depthStencilTextureWidth` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLSubImage/depthStencilTextureWidth)"]
    pub fn depth_stencil_texture_width(this: &XrWebGlSubImage) -> Option<u32>;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLSubImage",
        js_name = "depthStencilTextureHeight"
    )]
    #[doc = "Getter for the `depthStencilTextureHeight` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLSubImage/depthStencilTextureHeight)"]
    pub fn depth_stencil_texture_height(this: &XrWebGlSubImage) -> Option<u32>;
}
