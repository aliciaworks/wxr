#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRMediaQuadLayerInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrMediaQuadLayerInit` dictionary."]
    pub type XrMediaQuadLayerInit;
    #[doc = "Get the `invertStereo` field of this object."]
    #[wasm_bindgen(method, getter = "invertStereo")]
    pub fn get_invert_stereo(this: &XrMediaQuadLayerInit) -> Option<bool>;
    #[doc = "Change the `invertStereo` field of this object."]
    #[wasm_bindgen(method, setter = "invertStereo")]
    pub fn set_invert_stereo(this: &XrMediaQuadLayerInit, val: bool);
    #[doc = "Get the `layout` field of this object."]
    #[wasm_bindgen(method, getter = "layout")]
    pub fn get_layout(this: &XrMediaQuadLayerInit) -> Option<XrLayerLayout>;
    #[doc = "Change the `layout` field of this object."]
    #[wasm_bindgen(method, setter = "layout")]
    pub fn set_layout(this: &XrMediaQuadLayerInit, val: XrLayerLayout);
    #[doc = "Get the `space` field of this object."]
    #[wasm_bindgen(method, getter = "space")]
    pub fn get_space(this: &XrMediaQuadLayerInit) -> XrSpace;
    #[doc = "Change the `space` field of this object."]
    #[wasm_bindgen(method, setter = "space")]
    pub fn set_space(this: &XrMediaQuadLayerInit, val: &XrSpace);
    #[doc = "Get the `height` field of this object."]
    #[wasm_bindgen(method, getter = "height")]
    pub fn get_height(this: &XrMediaQuadLayerInit) -> Option<f32>;
    #[doc = "Change the `height` field of this object."]
    #[wasm_bindgen(method, setter = "height")]
    pub fn set_height(this: &XrMediaQuadLayerInit, val: Option<f32>);
    #[doc = "Get the `transform` field of this object."]
    #[wasm_bindgen(method, getter = "transform")]
    pub fn get_transform(this: &XrMediaQuadLayerInit) -> Option<XrRigidTransform>;
    #[doc = "Change the `transform` field of this object."]
    #[wasm_bindgen(method, setter = "transform")]
    pub fn set_transform(this: &XrMediaQuadLayerInit, val: Option<&XrRigidTransform>);
    #[doc = "Get the `width` field of this object."]
    #[wasm_bindgen(method, getter = "width")]
    pub fn get_width(this: &XrMediaQuadLayerInit) -> Option<f32>;
    #[doc = "Change the `width` field of this object."]
    #[wasm_bindgen(method, setter = "width")]
    pub fn set_width(this: &XrMediaQuadLayerInit, val: Option<f32>);
}
impl XrMediaQuadLayerInit {
    #[doc = "Construct a new `XrMediaQuadLayerInit`."]
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
    #[deprecated = "Use `set_height()` instead."]
    pub fn height(&mut self, val: Option<f32>) -> &mut Self {
        self.set_height(val);
        self
    }
    #[deprecated = "Use `set_transform()` instead."]
    pub fn transform(&mut self, val: Option<&XrRigidTransform>) -> &mut Self {
        self.set_transform(val);
        self
    }
    #[deprecated = "Use `set_width()` instead."]
    pub fn width(&mut self, val: Option<f32>) -> &mut Self {
        self.set_width(val);
        self
    }
}
