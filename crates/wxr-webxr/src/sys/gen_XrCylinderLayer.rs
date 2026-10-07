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
        js_name = "XRCylinderLayer",
        typescript_type = "XRCylinderLayer"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrCylinderLayer` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCylinderLayer)"]
    pub type XrCylinderLayer;
    #[wasm_bindgen(method, getter, js_class = "XRCylinderLayer", js_name = "space")]
    #[doc = "Getter for the `space` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCylinderLayer/space)"]
    pub fn space(this: &XrCylinderLayer) -> XrSpace;
    #[wasm_bindgen(method, setter, js_class = "XRCylinderLayer", js_name = "space")]
    #[doc = "Setter for the `space` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCylinderLayer/space)"]
    pub fn set_space(this: &XrCylinderLayer, value: &XrSpace);
    #[wasm_bindgen(method, getter, js_class = "XRCylinderLayer", js_name = "transform")]
    #[doc = "Getter for the `transform` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCylinderLayer/transform)"]
    pub fn transform(this: &XrCylinderLayer) -> XrRigidTransform;
    #[wasm_bindgen(method, setter, js_class = "XRCylinderLayer", js_name = "transform")]
    #[doc = "Setter for the `transform` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCylinderLayer/transform)"]
    pub fn set_transform(this: &XrCylinderLayer, value: &XrRigidTransform);
    #[wasm_bindgen(method, getter, js_class = "XRCylinderLayer", js_name = "radius")]
    #[doc = "Getter for the `radius` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCylinderLayer/radius)"]
    pub fn radius(this: &XrCylinderLayer) -> f32;
    #[wasm_bindgen(method, setter, js_class = "XRCylinderLayer", js_name = "radius")]
    #[doc = "Setter for the `radius` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCylinderLayer/radius)"]
    pub fn set_radius(this: &XrCylinderLayer, value: f32);
    #[wasm_bindgen(method, getter, js_class = "XRCylinderLayer", js_name = "centralAngle")]
    #[doc = "Getter for the `centralAngle` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCylinderLayer/centralAngle)"]
    pub fn central_angle(this: &XrCylinderLayer) -> f32;
    #[wasm_bindgen(method, setter, js_class = "XRCylinderLayer", js_name = "centralAngle")]
    #[doc = "Setter for the `centralAngle` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCylinderLayer/centralAngle)"]
    pub fn set_central_angle(this: &XrCylinderLayer, value: f32);
    #[wasm_bindgen(method, getter, js_class = "XRCylinderLayer", js_name = "aspectRatio")]
    #[doc = "Getter for the `aspectRatio` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCylinderLayer/aspectRatio)"]
    pub fn aspect_ratio(this: &XrCylinderLayer) -> f32;
    #[wasm_bindgen(method, setter, js_class = "XRCylinderLayer", js_name = "aspectRatio")]
    #[doc = "Setter for the `aspectRatio` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCylinderLayer/aspectRatio)"]
    pub fn set_aspect_ratio(this: &XrCylinderLayer, value: f32);
    #[wasm_bindgen(method, getter, js_class = "XRCylinderLayer", js_name = "onredraw")]
    #[doc = "Getter for the `onredraw` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCylinderLayer/onredraw)"]
    pub fn onredraw(this: &XrCylinderLayer) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, setter, js_class = "XRCylinderLayer", js_name = "onredraw")]
    #[doc = "Setter for the `onredraw` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCylinderLayer/onredraw)"]
    pub fn set_onredraw(this: &XrCylinderLayer, value: &::wasm_bindgen::JsValue);
}
