#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrCompositionLayer",
        extends = "XrLayer",
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "XRQuadLayer",
        typescript_type = "XRQuadLayer"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrQuadLayer` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRQuadLayer)"]
    pub type XrQuadLayer;
    #[wasm_bindgen(method, getter, js_class = "XRQuadLayer", js_name = "space")]
    #[doc = "Getter for the `space` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRQuadLayer/space)"]
    pub fn space(this: &XrQuadLayer) -> XrSpace;
    #[wasm_bindgen(method, setter, js_class = "XRQuadLayer", js_name = "space")]
    #[doc = "Setter for the `space` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRQuadLayer/space)"]
    pub fn set_space(this: &XrQuadLayer, value: &XrSpace);
    #[wasm_bindgen(method, getter, js_class = "XRQuadLayer", js_name = "transform")]
    #[doc = "Getter for the `transform` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRQuadLayer/transform)"]
    pub fn transform(this: &XrQuadLayer) -> XrRigidTransform;
    #[wasm_bindgen(method, setter, js_class = "XRQuadLayer", js_name = "transform")]
    #[doc = "Setter for the `transform` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRQuadLayer/transform)"]
    pub fn set_transform(this: &XrQuadLayer, value: &XrRigidTransform);
    #[wasm_bindgen(method, getter, js_class = "XRQuadLayer", js_name = "width")]
    #[doc = "Getter for the `width` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRQuadLayer/width)"]
    pub fn width(this: &XrQuadLayer) -> f32;
    #[wasm_bindgen(method, setter, js_class = "XRQuadLayer", js_name = "width")]
    #[doc = "Setter for the `width` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRQuadLayer/width)"]
    pub fn set_width(this: &XrQuadLayer, value: f32);
    #[wasm_bindgen(method, getter, js_class = "XRQuadLayer", js_name = "height")]
    #[doc = "Getter for the `height` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRQuadLayer/height)"]
    pub fn height(this: &XrQuadLayer) -> f32;
    #[wasm_bindgen(method, setter, js_class = "XRQuadLayer", js_name = "height")]
    #[doc = "Setter for the `height` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRQuadLayer/height)"]
    pub fn set_height(this: &XrQuadLayer, value: f32);
    #[wasm_bindgen(method, getter, js_class = "XRQuadLayer", js_name = "onredraw")]
    #[doc = "Getter for the `onredraw` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRQuadLayer/onredraw)"]
    pub fn onredraw(this: &XrQuadLayer) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, setter, js_class = "XRQuadLayer", js_name = "onredraw")]
    #[doc = "Setter for the `onredraw` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRQuadLayer/onredraw)"]
    pub fn set_onredraw(this: &XrQuadLayer, value: &::wasm_bindgen::JsValue);
}
