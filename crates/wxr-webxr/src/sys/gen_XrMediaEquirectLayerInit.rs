#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRMediaEquirectLayerInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrMediaEquirectLayerInit` dictionary."]
    pub type XrMediaEquirectLayerInit;
    #[doc = "Get the `invertStereo` field of this object."]
    #[wasm_bindgen(method, getter = "invertStereo")]
    pub fn get_invert_stereo(this: &XrMediaEquirectLayerInit) -> Option<bool>;
    #[doc = "Change the `invertStereo` field of this object."]
    #[wasm_bindgen(method, setter = "invertStereo")]
    pub fn set_invert_stereo(this: &XrMediaEquirectLayerInit, val: bool);
    #[doc = "Get the `layout` field of this object."]
    #[wasm_bindgen(method, getter = "layout")]
    pub fn get_layout(this: &XrMediaEquirectLayerInit) -> Option<XrLayerLayout>;
    #[doc = "Change the `layout` field of this object."]
    #[wasm_bindgen(method, setter = "layout")]
    pub fn set_layout(this: &XrMediaEquirectLayerInit, val: XrLayerLayout);
    #[doc = "Get the `space` field of this object."]
    #[wasm_bindgen(method, getter = "space")]
    pub fn get_space(this: &XrMediaEquirectLayerInit) -> XrSpace;
    #[doc = "Change the `space` field of this object."]
    #[wasm_bindgen(method, setter = "space")]
    pub fn set_space(this: &XrMediaEquirectLayerInit, val: &XrSpace);
    #[doc = "Get the `centralHorizontalAngle` field of this object."]
    #[wasm_bindgen(method, getter = "centralHorizontalAngle")]
    pub fn get_central_horizontal_angle(this: &XrMediaEquirectLayerInit) -> Option<f32>;
    #[doc = "Change the `centralHorizontalAngle` field of this object."]
    #[wasm_bindgen(method, setter = "centralHorizontalAngle")]
    pub fn set_central_horizontal_angle(this: &XrMediaEquirectLayerInit, val: f32);
    #[doc = "Get the `lowerVerticalAngle` field of this object."]
    #[wasm_bindgen(method, getter = "lowerVerticalAngle")]
    pub fn get_lower_vertical_angle(this: &XrMediaEquirectLayerInit) -> Option<f32>;
    #[doc = "Change the `lowerVerticalAngle` field of this object."]
    #[wasm_bindgen(method, setter = "lowerVerticalAngle")]
    pub fn set_lower_vertical_angle(this: &XrMediaEquirectLayerInit, val: f32);
    #[doc = "Get the `radius` field of this object."]
    #[wasm_bindgen(method, getter = "radius")]
    pub fn get_radius(this: &XrMediaEquirectLayerInit) -> Option<f32>;
    #[doc = "Change the `radius` field of this object."]
    #[wasm_bindgen(method, setter = "radius")]
    pub fn set_radius(this: &XrMediaEquirectLayerInit, val: f32);
    #[doc = "Get the `transform` field of this object."]
    #[wasm_bindgen(method, getter = "transform")]
    pub fn get_transform(this: &XrMediaEquirectLayerInit) -> Option<XrRigidTransform>;
    #[doc = "Change the `transform` field of this object."]
    #[wasm_bindgen(method, setter = "transform")]
    pub fn set_transform(this: &XrMediaEquirectLayerInit, val: Option<&XrRigidTransform>);
    #[doc = "Get the `upperVerticalAngle` field of this object."]
    #[wasm_bindgen(method, getter = "upperVerticalAngle")]
    pub fn get_upper_vertical_angle(this: &XrMediaEquirectLayerInit) -> Option<f32>;
    #[doc = "Change the `upperVerticalAngle` field of this object."]
    #[wasm_bindgen(method, setter = "upperVerticalAngle")]
    pub fn set_upper_vertical_angle(this: &XrMediaEquirectLayerInit, val: f32);
}
impl XrMediaEquirectLayerInit {
    #[doc = "Construct a new `XrMediaEquirectLayerInit`."]
    pub fn new(space: &XrSpace) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_space(space);
        ret
    }
    #[deprecated = "Use `set_invert_stereo()` instead."]
    pub fn invert_stereo(&mut self, val: bool) -> &mut Self {
        self.set_invert_stereo(val);
        self
    }
    #[deprecated = "Use `set_layout()` instead."]
    pub fn layout(&mut self, val: XrLayerLayout) -> &mut Self {
        self.set_layout(val);
        self
    }
    #[deprecated = "Use `set_space()` instead."]
    pub fn space(&mut self, val: &XrSpace) -> &mut Self {
        self.set_space(val);
        self
    }
    #[deprecated = "Use `set_central_horizontal_angle()` instead."]
    pub fn central_horizontal_angle(&mut self, val: f32) -> &mut Self {
        self.set_central_horizontal_angle(val);
        self
    }
    #[deprecated = "Use `set_lower_vertical_angle()` instead."]
    pub fn lower_vertical_angle(&mut self, val: f32) -> &mut Self {
        self.set_lower_vertical_angle(val);
        self
    }
    #[deprecated = "Use `set_radius()` instead."]
    pub fn radius(&mut self, val: f32) -> &mut Self {
        self.set_radius(val);
        self
    }
    #[deprecated = "Use `set_transform()` instead."]
    pub fn transform(&mut self, val: Option<&XrRigidTransform>) -> &mut Self {
        self.set_transform(val);
        self
    }
    #[deprecated = "Use `set_upper_vertical_angle()` instead."]
    pub fn upper_vertical_angle(&mut self, val: f32) -> &mut Self {
        self.set_upper_vertical_angle(val);
        self
    }
}
