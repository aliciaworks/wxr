#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRPermissionDescriptor")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrPermissionDescriptor` dictionary."]
    pub type XrPermissionDescriptor;
    #[doc = "Get the `name` field of this object."]
    #[wasm_bindgen(method, getter = "name")]
    pub fn get_name(this: &XrPermissionDescriptor) -> ::alloc::string::String;
    #[doc = "Change the `name` field of this object."]
    #[wasm_bindgen(method, setter = "name")]
    pub fn set_name(this: &XrPermissionDescriptor, val: &str);
    #[doc = "Get the `mode` field of this object."]
    #[wasm_bindgen(method, getter = "mode")]
    pub fn get_mode(this: &XrPermissionDescriptor) -> Option<XrSessionMode>;
    #[doc = "Change the `mode` field of this object."]
    #[wasm_bindgen(method, setter = "mode")]
    pub fn set_mode(this: &XrPermissionDescriptor, val: XrSessionMode);
    #[doc = "Get the `optionalFeatures` field of this object."]
    #[wasm_bindgen(method, getter = "optionalFeatures")]
    pub fn get_optional_features(this: &XrPermissionDescriptor) -> Option<::js_sys::Array>;
    #[doc = "Change the `optionalFeatures` field of this object."]
    #[wasm_bindgen(method, setter = "optionalFeatures")]
    pub fn set_optional_features(this: &XrPermissionDescriptor, val: &::wasm_bindgen::JsValue);
    #[doc = "Get the `requiredFeatures` field of this object."]
    #[wasm_bindgen(method, getter = "requiredFeatures")]
    pub fn get_required_features(this: &XrPermissionDescriptor) -> Option<::js_sys::Array>;
    #[doc = "Change the `requiredFeatures` field of this object."]
    #[wasm_bindgen(method, setter = "requiredFeatures")]
    pub fn set_required_features(this: &XrPermissionDescriptor, val: &::wasm_bindgen::JsValue);
}
impl XrPermissionDescriptor {
    #[doc = "Construct a new `XrPermissionDescriptor`."]
    pub fn new(name: &str) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_name(name);
        ret
    }
    #[deprecated = "Use `set_name()` instead."]
    pub fn name(&mut self, val: &str) -> &mut Self {
        self.set_name(val);
        self
    }
    #[deprecated = "Use `set_mode()` instead."]
    pub fn mode(&mut self, val: XrSessionMode) -> &mut Self {
        self.set_mode(val);
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
