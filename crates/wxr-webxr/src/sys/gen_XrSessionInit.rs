#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRSessionInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrSessionInit` dictionary."]
    pub type XrSessionInit;
    #[doc = "Get the `depthSensing` field of this object."]
    #[wasm_bindgen(method, getter = "depthSensing")]
    pub fn get_depth_sensing(this: &XrSessionInit) -> Option<XrDepthStateInit>;
    #[doc = "Change the `depthSensing` field of this object."]
    #[wasm_bindgen(method, setter = "depthSensing")]
    pub fn set_depth_sensing(this: &XrSessionInit, val: &XrDepthStateInit);
    #[doc = "Get the `domOverlay` field of this object."]
    #[wasm_bindgen(method, getter = "domOverlay")]
    pub fn get_dom_overlay(this: &XrSessionInit) -> Option<XrdomOverlayInit>;
    #[doc = "Change the `domOverlay` field of this object."]
    #[wasm_bindgen(method, setter = "domOverlay")]
    pub fn set_dom_overlay(this: &XrSessionInit, val: Option<&XrdomOverlayInit>);
    #[doc = "Get the `optionalFeatures` field of this object."]
    #[wasm_bindgen(method, getter = "optionalFeatures")]
    pub fn get_optional_features(this: &XrSessionInit) -> Option<::js_sys::Array>;
    #[doc = "Change the `optionalFeatures` field of this object."]
    #[wasm_bindgen(method, setter = "optionalFeatures")]
    pub fn set_optional_features(this: &XrSessionInit, val: &::wasm_bindgen::JsValue);
    #[doc = "Get the `requiredFeatures` field of this object."]
    #[wasm_bindgen(method, getter = "requiredFeatures")]
    pub fn get_required_features(this: &XrSessionInit) -> Option<::js_sys::Array>;
    #[doc = "Change the `requiredFeatures` field of this object."]
    #[wasm_bindgen(method, setter = "requiredFeatures")]
    pub fn set_required_features(this: &XrSessionInit, val: &::wasm_bindgen::JsValue);
}
impl XrSessionInit {
    #[doc = "Construct a new `XrSessionInit`."]
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret
    }
    #[deprecated = "Use `set_depth_sensing()` instead."]
    pub fn depth_sensing(&mut self, val: &XrDepthStateInit) -> &mut Self {
        self.set_depth_sensing(val);
        self
    }
    #[deprecated = "Use `set_dom_overlay()` instead."]
    pub fn dom_overlay(&mut self, val: Option<&XrdomOverlayInit>) -> &mut Self {
        self.set_dom_overlay(val);
        self
    }
    #[deprecated = "Use `set_optional_features()` instead."]
    pub fn optional_features(&mut self, val: &::wasm_bindgen::JsValue) -> &mut Self {
        self.set_optional_features(val);
        self
    }
    #[deprecated = "Use `set_required_features()` instead."]
    pub fn required_features(&mut self, val: &::wasm_bindgen::JsValue) -> &mut Self {
        self.set_required_features(val);
        self
    }
}
impl Default for XrSessionInit {
    fn default() -> Self {
        Self::new()
    }
}
