#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRProjectionLayerInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrProjectionLayerInit` dictionary."]
    pub type XrProjectionLayerInit;
    #[doc = "Get the `clearOnAccess` field of this object."]
    #[wasm_bindgen(method, getter = "clearOnAccess")]
    pub fn get_clear_on_access(this: &XrProjectionLayerInit) -> Option<bool>;
    #[doc = "Change the `clearOnAccess` field of this object."]
    #[wasm_bindgen(method, setter = "clearOnAccess")]
    pub fn set_clear_on_access(this: &XrProjectionLayerInit, val: bool);
    #[doc = "Get the `colorFormat` field of this object."]
    #[wasm_bindgen(method, getter = "colorFormat")]
    pub fn get_color_format(this: &XrProjectionLayerInit) -> Option<u32>;
    #[doc = "Change the `colorFormat` field of this object."]
    #[wasm_bindgen(method, setter = "colorFormat")]
    pub fn set_color_format(this: &XrProjectionLayerInit, val: u32);
    #[doc = "Get the `depthFormat` field of this object."]
    #[wasm_bindgen(method, getter = "depthFormat")]
    pub fn get_depth_format(this: &XrProjectionLayerInit) -> Option<u32>;
    #[doc = "Change the `depthFormat` field of this object."]
    #[wasm_bindgen(method, setter = "depthFormat")]
    pub fn set_depth_format(this: &XrProjectionLayerInit, val: u32);
    #[doc = "Get the `scaleFactor` field of this object."]
    #[wasm_bindgen(method, getter = "scaleFactor")]
    pub fn get_scale_factor(this: &XrProjectionLayerInit) -> Option<f64>;
    #[doc = "Change the `scaleFactor` field of this object."]
    #[wasm_bindgen(method, setter = "scaleFactor")]
    pub fn set_scale_factor(this: &XrProjectionLayerInit, val: f64);
    #[doc = "Get the `textureType` field of this object."]
    #[wasm_bindgen(method, getter = "textureType")]
    pub fn get_texture_type(this: &XrProjectionLayerInit) -> Option<XrTextureType>;
    #[doc = "Change the `textureType` field of this object."]
    #[wasm_bindgen(method, setter = "textureType")]
    pub fn set_texture_type(this: &XrProjectionLayerInit, val: XrTextureType);
}
impl XrProjectionLayerInit {
    #[doc = "Construct a new `XrProjectionLayerInit`."]
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret
    }
    #[deprecated = "Use `set_clear_on_access()` instead."]
    pub fn clear_on_access(&mut self, val: bool) -> &mut Self {
        self.set_clear_on_access(val);
        self
    }
    #[deprecated = "Use `set_color_format()` instead."]
    pub fn color_format(&mut self, val: u32) -> &mut Self {
        self.set_color_format(val);
        self
    }
    #[deprecated = "Use `set_depth_format()` instead."]
    pub fn depth_format(&mut self, val: u32) -> &mut Self {
        self.set_depth_format(val);
        self
    }
    #[deprecated = "Use `set_scale_factor()` instead."]
    pub fn scale_factor(&mut self, val: f64) -> &mut Self {
        self.set_scale_factor(val);
        self
    }
    #[deprecated = "Use `set_texture_type()` instead."]
    pub fn texture_type(&mut self, val: XrTextureType) -> &mut Self {
        self.set_texture_type(val);
        self
    }
}
impl Default for XrProjectionLayerInit {
    fn default() -> Self {
        Self::new()
    }
}
