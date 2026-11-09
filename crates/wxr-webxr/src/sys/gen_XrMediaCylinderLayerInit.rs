#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRMediaCylinderLayerInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrMediaCylinderLayerInit` dictionary."]
    pub type XrMediaCylinderLayerInit;
    #[doc = "Get the `invertStereo` field of this object."]
    #[wasm_bindgen(method, getter = "invertStereo")]
    pub fn get_invert_stereo(this: &XrMediaCylinderLayerInit) -> Option<bool>;
    #[doc = "Change the `invertStereo` field of this object."]
    #[wasm_bindgen(method, setter = "invertStereo")]
    pub fn set_invert_stereo(this: &XrMediaCylinderLayerInit, val: bool);
    #[doc = "Get the `layout` field of this object."]
    #[wasm_bindgen(method, getter = "layout")]
    pub fn get_layout(this: &XrMediaCylinderLayerInit) -> Option<XrLayerLayout>;
    #[doc = "Change the `layout` field of this object."]
    #[wasm_bindgen(method, setter = "layout")]
    pub fn set_layout(this: &XrMediaCylinderLayerInit, val: XrLayerLayout);
    #[doc = "Get the `space` field of this object."]
    #[wasm_bindgen(method, getter = "space")]
    pub fn get_space(this: &XrMediaCylinderLayerInit) -> XrSpace;
    #[doc = "Change the `space` field of this object."]
    #[wasm_bindgen(method, setter = "space")]
    pub fn set_space(this: &XrMediaCylinderLayerInit, val: &XrSpace);
    #[doc = "Get the `aspectRatio` field of this object."]
    #[wasm_bindgen(method, getter = "aspectRatio")]
    pub fn get_aspect_ratio(this: &XrMediaCylinderLayerInit) -> Option<f32>;
    #[doc = "Change the `aspectRatio` field of this object."]
    #[wasm_bindgen(method, setter = "aspectRatio")]
    pub fn set_aspect_ratio(this: &XrMediaCylinderLayerInit, val: Option<f32>);
    #[doc = "Get the `centralAngle` field of this object."]
    #[wasm_bindgen(method, getter = "centralAngle")]
    pub fn get_central_angle(this: &XrMediaCylinderLayerInit) -> Option<f32>;
    #[doc = "Change the `centralAngle` field of this object."]
    #[wasm_bindgen(method, setter = "centralAngle")]
    pub fn set_central_angle(this: &XrMediaCylinderLayerInit, val: f32);
    #[doc = "Get the `radius` field of this object."]
    #[wasm_bindgen(method, getter = "radius")]
    pub fn get_radius(this: &XrMediaCylinderLayerInit) -> Option<f32>;
    #[doc = "Change the `radius` field of this object."]
    #[wasm_bindgen(method, setter = "radius")]
    pub fn set_radius(this: &XrMediaCylinderLayerInit, val: f32);
    #[doc = "Get the `transform` field of this object."]
    #[wasm_bindgen(method, getter = "transform")]
    pub fn get_transform(this: &XrMediaCylinderLayerInit) -> Option<XrRigidTransform>;
    #[doc = "Change the `transform` field of this object."]
    #[wasm_bindgen(method, setter = "transform")]
    pub fn set_transform(this: &XrMediaCylinderLayerInit, val: Option<&XrRigidTransform>);
}
impl XrMediaCylinderLayerInit {
    #[doc = "Construct a new `XrMediaCylinderLayerInit`."]
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
    #[deprecated = "Use `set_aspect_ratio()` instead."]
    pub fn aspect_ratio(&mut self, val: Option<f32>) -> &mut Self {
        self.set_aspect_ratio(val);
        self
    }
    #[deprecated = "Use `set_central_angle()` instead."]
    pub fn central_angle(&mut self, val: f32) -> &mut Self {
        self.set_central_angle(val);
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
}
