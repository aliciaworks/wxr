#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRGPUProjectionLayerInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrgpuProjectionLayerInit` dictionary."]
    pub type XrgpuProjectionLayerInit;
    #[doc = "Get the `colorFormat` field of this object."]
    #[wasm_bindgen(method, getter = "colorFormat")]
    pub fn get_color_format(this: &XrgpuProjectionLayerInit) -> ::alloc::string::String;
    #[doc = "Change the `colorFormat` field of this object."]
    #[wasm_bindgen(method, setter = "colorFormat")]
    pub fn set_color_format(this: &XrgpuProjectionLayerInit, val: &str);
    #[doc = "Get the `depthStencilFormat` field of this object."]
    #[wasm_bindgen(method, getter = "depthStencilFormat")]
    pub fn get_depth_stencil_format(
        this: &XrgpuProjectionLayerInit,
    ) -> Option<::alloc::string::String>;
    #[doc = "Change the `depthStencilFormat` field of this object."]
    #[wasm_bindgen(method, setter = "depthStencilFormat")]
    pub fn set_depth_stencil_format(this: &XrgpuProjectionLayerInit, val: Option<&str>);
    #[doc = "Get the `scaleFactor` field of this object."]
    #[wasm_bindgen(method, getter = "scaleFactor")]
    pub fn get_scale_factor(this: &XrgpuProjectionLayerInit) -> Option<f64>;
    #[doc = "Change the `scaleFactor` field of this object."]
    #[wasm_bindgen(method, setter = "scaleFactor")]
    pub fn set_scale_factor(this: &XrgpuProjectionLayerInit, val: f64);
    #[doc = "Get the `textureUsage` field of this object."]
    #[wasm_bindgen(method, getter = "textureUsage")]
    pub fn get_texture_usage(this: &XrgpuProjectionLayerInit) -> Option<u32>;
    #[doc = "Change the `textureUsage` field of this object."]
    #[wasm_bindgen(method, setter = "textureUsage")]
    pub fn set_texture_usage(this: &XrgpuProjectionLayerInit, val: u32);
}
impl XrgpuProjectionLayerInit {
    #[doc = "Construct a new `XrgpuProjectionLayerInit`."]
    pub fn new(color_format: &str) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_color_format(color_format);
        ret
    }
    #[deprecated = "Use `set_color_format()` instead."]
    pub fn color_format(&mut self, val: &str) -> &mut Self {
        self.set_color_format(val);
        self
    }
    #[deprecated = "Use `set_depth_stencil_format()` instead."]
    pub fn depth_stencil_format(&mut self, val: Option<&str>) -> &mut Self {
        self.set_depth_stencil_format(val);
        self
    }
    #[deprecated = "Use `set_scale_factor()` instead."]
    pub fn scale_factor(&mut self, val: f64) -> &mut Self {
        self.set_scale_factor(val);
        self
    }
    #[deprecated = "Use `set_texture_usage()` instead."]
    pub fn texture_usage(&mut self, val: u32) -> &mut Self {
        self.set_texture_usage(val);
        self
    }
}
