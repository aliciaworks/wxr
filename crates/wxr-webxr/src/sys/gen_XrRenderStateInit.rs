#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRRenderStateInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrRenderStateInit` dictionary."]
    pub type XrRenderStateInit;
    #[doc = "Get the `baseLayer` field of this object."]
    #[wasm_bindgen(method, getter = "baseLayer")]
    pub fn get_base_layer(this: &XrRenderStateInit) -> Option<XrWebGlLayer>;
    #[doc = "Change the `baseLayer` field of this object."]
    #[wasm_bindgen(method, setter = "baseLayer")]
    pub fn set_base_layer(this: &XrRenderStateInit, val: Option<&XrWebGlLayer>);
    #[doc = "Get the `depthFar` field of this object."]
    #[wasm_bindgen(method, getter = "depthFar")]
    pub fn get_depth_far(this: &XrRenderStateInit) -> Option<f64>;
    #[doc = "Change the `depthFar` field of this object."]
    #[wasm_bindgen(method, setter = "depthFar")]
    pub fn set_depth_far(this: &XrRenderStateInit, val: f64);
    #[doc = "Get the `depthNear` field of this object."]
    #[wasm_bindgen(method, getter = "depthNear")]
    pub fn get_depth_near(this: &XrRenderStateInit) -> Option<f64>;
    #[doc = "Change the `depthNear` field of this object."]
    #[wasm_bindgen(method, setter = "depthNear")]
    pub fn set_depth_near(this: &XrRenderStateInit, val: f64);
    #[doc = "Get the `inlineVerticalFieldOfView` field of this object."]
    #[wasm_bindgen(method, getter = "inlineVerticalFieldOfView")]
    pub fn get_inline_vertical_field_of_view(this: &XrRenderStateInit) -> Option<f64>;
    #[doc = "Change the `inlineVerticalFieldOfView` field of this object."]
    #[wasm_bindgen(method, setter = "inlineVerticalFieldOfView")]
    pub fn set_inline_vertical_field_of_view(this: &XrRenderStateInit, val: f64);
    #[doc = "Get the `layers` field of this object."]
    #[wasm_bindgen(method, getter = "layers")]
    pub fn get_layers(this: &XrRenderStateInit) -> Option<::js_sys::Array>;
    #[doc = "Change the `layers` field of this object."]
    #[wasm_bindgen(method, setter = "layers")]
    pub fn set_layers(this: &XrRenderStateInit, val: &::wasm_bindgen::JsValue);
    #[doc = "Get the `passthroughFullyObscured` field of this object."]
    #[wasm_bindgen(method, getter = "passthroughFullyObscured")]
    pub fn get_passthrough_fully_obscured(this: &XrRenderStateInit) -> Option<bool>;
    #[doc = "Change the `passthroughFullyObscured` field of this object."]
    #[wasm_bindgen(method, setter = "passthroughFullyObscured")]
    pub fn set_passthrough_fully_obscured(this: &XrRenderStateInit, val: bool);
}
impl XrRenderStateInit {
    #[doc = "Construct a new `XrRenderStateInit`."]
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret
    }
    #[deprecated = "Use `set_base_layer()` instead."]
    pub fn base_layer(&mut self, val: Option<&XrWebGlLayer>) -> &mut Self {
        self.set_base_layer(val);
        self
    }
    #[deprecated = "Use `set_depth_far()` instead."]
    pub fn depth_far(&mut self, val: f64) -> &mut Self {
        self.set_depth_far(val);
        self
    }
    #[deprecated = "Use `set_depth_near()` instead."]
    pub fn depth_near(&mut self, val: f64) -> &mut Self {
        self.set_depth_near(val);
        self
    }
    #[deprecated = "Use `set_inline_vertical_field_of_view()` instead."]
    pub fn inline_vertical_field_of_view(&mut self, val: f64) -> &mut Self {
        self.set_inline_vertical_field_of_view(val);
        self
    }
    #[deprecated = "Use `set_layers()` instead."]
    pub fn layers(&mut self, val: Option<&::wasm_bindgen::JsValue>) -> &mut Self {
        self.set_layers(val.unwrap_or(&::wasm_bindgen::JsValue::NULL));
        self
    }
    #[deprecated = "Use `set_passthrough_fully_obscured()` instead."]
    pub fn passthrough_fully_obscured(&mut self, val: bool) -> &mut Self {
        self.set_passthrough_fully_obscured(val);
        self
    }
}
impl Default for XrRenderStateInit {
    fn default() -> Self {
        Self::new()
    }
}
