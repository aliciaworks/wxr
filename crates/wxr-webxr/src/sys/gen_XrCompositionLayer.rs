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
        js_name = "XRCompositionLayer",
        typescript_type = "XRCompositionLayer"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrCompositionLayer` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCompositionLayer)"]
    pub type XrCompositionLayer;
    #[wasm_bindgen(method, getter, js_class = "XRCompositionLayer", js_name = "layout")]
    #[doc = "Getter for the `layout` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCompositionLayer/layout)"]
    pub fn layout(this: &XrCompositionLayer) -> XrLayerLayout;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRCompositionLayer",
        js_name = "blendTextureSourceAlpha"
    )]
    #[doc = "Getter for the `blendTextureSourceAlpha` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCompositionLayer/blendTextureSourceAlpha)"]
    pub fn blend_texture_source_alpha(this: &XrCompositionLayer) -> bool;
    #[wasm_bindgen(
        method,
        setter,
        js_class = "XRCompositionLayer",
        js_name = "blendTextureSourceAlpha"
    )]
    #[doc = "Setter for the `blendTextureSourceAlpha` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCompositionLayer/blendTextureSourceAlpha)"]
    pub fn set_blend_texture_source_alpha(this: &XrCompositionLayer, value: bool);
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRCompositionLayer",
        js_name = "forceMonoPresentation"
    )]
    #[doc = "Getter for the `forceMonoPresentation` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCompositionLayer/forceMonoPresentation)"]
    pub fn force_mono_presentation(this: &XrCompositionLayer) -> bool;
    #[wasm_bindgen(
        method,
        setter,
        js_class = "XRCompositionLayer",
        js_name = "forceMonoPresentation"
    )]
    #[doc = "Setter for the `forceMonoPresentation` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCompositionLayer/forceMonoPresentation)"]
    pub fn set_force_mono_presentation(this: &XrCompositionLayer, value: bool);
    #[wasm_bindgen(method, getter, js_class = "XRCompositionLayer", js_name = "opacity")]
    #[doc = "Getter for the `opacity` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCompositionLayer/opacity)"]
    pub fn opacity(this: &XrCompositionLayer) -> f32;
    #[wasm_bindgen(method, setter, js_class = "XRCompositionLayer", js_name = "opacity")]
    #[doc = "Setter for the `opacity` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCompositionLayer/opacity)"]
    pub fn set_opacity(this: &XrCompositionLayer, value: f32);
    #[wasm_bindgen(method, getter, js_class = "XRCompositionLayer", js_name = "mipLevels")]
    #[doc = "Getter for the `mipLevels` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCompositionLayer/mipLevels)"]
    pub fn mip_levels(this: &XrCompositionLayer) -> u32;
    #[wasm_bindgen(method, getter, js_class = "XRCompositionLayer", js_name = "quality")]
    #[doc = "Getter for the `quality` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCompositionLayer/quality)"]
    pub fn quality(this: &XrCompositionLayer) -> XrLayerQuality;
    #[wasm_bindgen(method, setter, js_class = "XRCompositionLayer", js_name = "quality")]
    #[doc = "Setter for the `quality` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCompositionLayer/quality)"]
    pub fn set_quality(this: &XrCompositionLayer, value: XrLayerQuality);
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRCompositionLayer",
        js_name = "needsRedraw"
    )]
    #[doc = "Getter for the `needsRedraw` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCompositionLayer/needsRedraw)"]
    pub fn needs_redraw(this: &XrCompositionLayer) -> bool;
    #[wasm_bindgen(method, js_class = "XRCompositionLayer")]
    #[doc = "The `destroy()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCompositionLayer/destroy)"]
    pub fn destroy(this: &XrCompositionLayer);
}
