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
        js_name = "XREquirectLayer",
        typescript_type = "XREquirectLayer"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrEquirectLayer` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer)"]
    pub type XrEquirectLayer;
    #[wasm_bindgen(method, getter, js_class = "XREquirectLayer", js_name = "space")]
    #[doc = "Getter for the `space` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/space)"]
    pub fn space(this: &XrEquirectLayer) -> XrSpace;
    #[wasm_bindgen(method, setter, js_class = "XREquirectLayer", js_name = "space")]
    #[doc = "Setter for the `space` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/space)"]
    pub fn set_space(this: &XrEquirectLayer, value: &XrSpace);
    #[wasm_bindgen(method, getter, js_class = "XREquirectLayer", js_name = "transform")]
    #[doc = "Getter for the `transform` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/transform)"]
    pub fn transform(this: &XrEquirectLayer) -> XrRigidTransform;
    #[wasm_bindgen(method, setter, js_class = "XREquirectLayer", js_name = "transform")]
    #[doc = "Setter for the `transform` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/transform)"]
    pub fn set_transform(this: &XrEquirectLayer, value: &XrRigidTransform);
    #[wasm_bindgen(method, getter, js_class = "XREquirectLayer", js_name = "radius")]
    #[doc = "Getter for the `radius` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/radius)"]
    pub fn radius(this: &XrEquirectLayer) -> f32;
    #[wasm_bindgen(method, setter, js_class = "XREquirectLayer", js_name = "radius")]
    #[doc = "Setter for the `radius` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/radius)"]
    pub fn set_radius(this: &XrEquirectLayer, value: f32);
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XREquirectLayer",
        js_name = "centralHorizontalAngle"
    )]
    #[doc = "Getter for the `centralHorizontalAngle` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/centralHorizontalAngle)"]
    pub fn central_horizontal_angle(this: &XrEquirectLayer) -> f32;
    #[wasm_bindgen(
        method,
        setter,
        js_class = "XREquirectLayer",
        js_name = "centralHorizontalAngle"
    )]
    #[doc = "Setter for the `centralHorizontalAngle` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/centralHorizontalAngle)"]
    pub fn set_central_horizontal_angle(this: &XrEquirectLayer, value: f32);
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XREquirectLayer",
        js_name = "upperVerticalAngle"
    )]
    #[doc = "Getter for the `upperVerticalAngle` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/upperVerticalAngle)"]
    pub fn upper_vertical_angle(this: &XrEquirectLayer) -> f32;
    #[wasm_bindgen(
        method,
        setter,
        js_class = "XREquirectLayer",
        js_name = "upperVerticalAngle"
    )]
    #[doc = "Setter for the `upperVerticalAngle` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/upperVerticalAngle)"]
    pub fn set_upper_vertical_angle(this: &XrEquirectLayer, value: f32);
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XREquirectLayer",
        js_name = "lowerVerticalAngle"
    )]
    #[doc = "Getter for the `lowerVerticalAngle` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/lowerVerticalAngle)"]
    pub fn lower_vertical_angle(this: &XrEquirectLayer) -> f32;
    #[wasm_bindgen(
        method,
        setter,
        js_class = "XREquirectLayer",
        js_name = "lowerVerticalAngle"
    )]
    #[doc = "Setter for the `lowerVerticalAngle` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/lowerVerticalAngle)"]
    pub fn set_lower_vertical_angle(this: &XrEquirectLayer, value: f32);
    #[wasm_bindgen(method, getter, js_class = "XREquirectLayer", js_name = "onredraw")]
    #[doc = "Getter for the `onredraw` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/onredraw)"]
    pub fn onredraw(this: &XrEquirectLayer) -> Option<::js_sys::Function>;
    #[wasm_bindgen(method, setter, js_class = "XREquirectLayer", js_name = "onredraw")]
    #[doc = "Setter for the `onredraw` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XREquirectLayer/onredraw)"]
    pub fn set_onredraw(this: &XrEquirectLayer, value: Option<&::js_sys::Function>);
}
