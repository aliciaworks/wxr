#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRHitTestOptionsInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrHitTestOptionsInit` dictionary."]
    pub type XrHitTestOptionsInit;
    #[doc = "Get the `entityTypes` field of this object."]
    #[wasm_bindgen(method, getter = "entityTypes")]
    pub fn get_entity_types(this: &XrHitTestOptionsInit) -> Option<::js_sys::Array>;
    #[doc = "Change the `entityTypes` field of this object."]
    #[wasm_bindgen(method, setter = "entityTypes")]
    pub fn set_entity_types(this: &XrHitTestOptionsInit, val: &::wasm_bindgen::JsValue);
    #[doc = "Get the `offsetRay` field of this object."]
    #[wasm_bindgen(method, getter = "offsetRay")]
    pub fn get_offset_ray(this: &XrHitTestOptionsInit) -> Option<XrRay>;
    #[doc = "Change the `offsetRay` field of this object."]
    #[wasm_bindgen(method, setter = "offsetRay")]
    pub fn set_offset_ray(this: &XrHitTestOptionsInit, val: &XrRay);
    #[doc = "Get the `space` field of this object."]
    #[wasm_bindgen(method, getter = "space")]
    pub fn get_space(this: &XrHitTestOptionsInit) -> XrSpace;
    #[doc = "Change the `space` field of this object."]
    #[wasm_bindgen(method, setter = "space")]
    pub fn set_space(this: &XrHitTestOptionsInit, val: &XrSpace);
}
impl XrHitTestOptionsInit {
    #[doc = "Construct a new `XrHitTestOptionsInit`."]
    pub fn new(space: &XrSpace) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_space(space);
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
    #[deprecated = "Use `set_space()` instead."]
    pub fn space(&mut self, val: &XrSpace) -> &mut Self {
        self.set_space(val);
        self
    }
}
