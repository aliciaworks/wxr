#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRTransientInputHitTestOptionsInit"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrTransientInputHitTestOptionsInit` dictionary."]
    pub type XrTransientInputHitTestOptionsInit;
    #[doc = "Get the `entityTypes` field of this object."]
    #[wasm_bindgen(method, getter = "entityTypes")]
    pub fn get_entity_types(this: &XrTransientInputHitTestOptionsInit) -> Option<::js_sys::Array>;
    #[doc = "Change the `entityTypes` field of this object."]
    #[wasm_bindgen(method, setter = "entityTypes")]
    pub fn set_entity_types(
        this: &XrTransientInputHitTestOptionsInit,
        val: &::wasm_bindgen::JsValue,
    );
    #[doc = "Get the `offsetRay` field of this object."]
    #[wasm_bindgen(method, getter = "offsetRay")]
    pub fn get_offset_ray(this: &XrTransientInputHitTestOptionsInit) -> Option<XrRay>;
    #[doc = "Change the `offsetRay` field of this object."]
    #[wasm_bindgen(method, setter = "offsetRay")]
    pub fn set_offset_ray(this: &XrTransientInputHitTestOptionsInit, val: &XrRay);
    #[doc = "Get the `profile` field of this object."]
    #[wasm_bindgen(method, getter = "profile")]
    pub fn get_profile(this: &XrTransientInputHitTestOptionsInit) -> ::alloc::string::String;
    #[doc = "Change the `profile` field of this object."]
    #[wasm_bindgen(method, setter = "profile")]
    pub fn set_profile(this: &XrTransientInputHitTestOptionsInit, val: &str);
}
impl XrTransientInputHitTestOptionsInit {
    #[doc = "Construct a new `XrTransientInputHitTestOptionsInit`."]
    pub fn new(profile: &str) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_profile(profile);
        ret
    }
    #[deprecated = "Use `set_entity_types()` instead."]
    pub fn entity_types(&mut self, val: &::wasm_bindgen::JsValue) -> &mut Self {
        self.set_entity_types(val);
        self
    }
    #[deprecated = "Use `set_offset_ray()` instead."]
    pub fn offset_ray(&mut self, val: &XrRay) -> &mut Self {
        self.set_offset_ray(val);
        self
    }
    #[deprecated = "Use `set_profile()` instead."]
    pub fn profile(&mut self, val: &str) -> &mut Self {
        self.set_profile(val);
        self
    }
}
