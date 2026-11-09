#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrLayer",
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "XRWebGLLayer",
        typescript_type = "XRWebGLLayer"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrWebGlLayer` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLLayer)"]
    pub type XrWebGlLayer;
    #[wasm_bindgen(method, getter, js_class = "XRWebGLLayer", js_name = "antialias")]
    #[doc = "Getter for the `antialias` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLLayer/antialias)"]
    pub fn antialias(this: &XrWebGlLayer) -> bool;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLLayer",
        js_name = "ignoreDepthValues"
    )]
    #[doc = "Getter for the `ignoreDepthValues` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLLayer/ignoreDepthValues)"]
    pub fn ignore_depth_values(this: &XrWebGlLayer) -> bool;
    #[wasm_bindgen(method, getter, js_class = "XRWebGLLayer", js_name = "fixedFoveation")]
    #[doc = "Getter for the `fixedFoveation` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLLayer/fixedFoveation)"]
    pub fn fixed_foveation(this: &XrWebGlLayer) -> Option<f32>;
    #[wasm_bindgen(method, setter, js_class = "XRWebGLLayer", js_name = "fixedFoveation")]
    #[doc = "Setter for the `fixedFoveation` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLLayer/fixedFoveation)"]
    pub fn set_fixed_foveation(this: &XrWebGlLayer, value: Option<f32>);
    #[wasm_bindgen(method, getter, js_class = "XRWebGLLayer", js_name = "framebuffer")]
    #[doc = "Getter for the `framebuffer` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLLayer/framebuffer)"]
    pub fn framebuffer(this: &XrWebGlLayer) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLLayer",
        js_name = "framebufferWidth"
    )]
    #[doc = "Getter for the `framebufferWidth` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLLayer/framebufferWidth)"]
    pub fn framebuffer_width(this: &XrWebGlLayer) -> u32;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRWebGLLayer",
        js_name = "framebufferHeight"
    )]
    #[doc = "Getter for the `framebufferHeight` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLLayer/framebufferHeight)"]
    pub fn framebuffer_height(this: &XrWebGlLayer) -> u32;
    #[wasm_bindgen(catch, constructor, js_class = "XRWebGLLayer")]
    #[doc = "The `new XrWebGlLayer(..)` constructor, creating a new instance of `XrWebGlLayer`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLLayer/XRWebGLLayer)"]
    pub fn new(
        session: &XrSession,
        context: &::wasm_bindgen::JsValue,
    ) -> Result<XrWebGlLayer, JsValue>;
    #[wasm_bindgen(catch, constructor, js_class = "XRWebGLLayer")]
    #[doc = "The `new XrWebGlLayer(..)` constructor, creating a new instance of `XrWebGlLayer`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLLayer/XRWebGLLayer)"]
    pub fn new_with_layer_init(
        session: &XrSession,
        context: &::wasm_bindgen::JsValue,
        layer_init: &XrWebGlLayerInit,
    ) -> Result<XrWebGlLayer, JsValue>;
    #[wasm_bindgen(
        static_method_of = "XrWebGlLayer",
        js_class = "XRWebGLLayer",
        js_name = "getNativeFramebufferScaleFactor"
    )]
    #[doc = "The `getNativeFramebufferScaleFactor()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLLayer/getNativeFramebufferScaleFactor_static)"]
    pub fn get_native_framebuffer_scale_factor(session: &XrSession) -> f64;
    #[wasm_bindgen(method, js_class = "XRWebGLLayer", js_name = "getViewport")]
    #[doc = "The `getViewport()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRWebGLLayer/getViewport)"]
    pub fn get_viewport(this: &XrWebGlLayer, view: &XrView) -> Option<XrViewport>;
}
