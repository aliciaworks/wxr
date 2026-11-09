#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRTrackedImageInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrTrackedImageInit` dictionary."]
    pub type XrTrackedImageInit;
    #[doc = "Get the `image` field of this object."]
    #[wasm_bindgen(method, getter = "image")]
    pub fn get_image(this: &XrTrackedImageInit) -> ::wasm_bindgen::JsValue;
    #[doc = "Change the `image` field of this object."]
    #[wasm_bindgen(method, setter = "image")]
    pub fn set_image(this: &XrTrackedImageInit, val: &::wasm_bindgen::JsValue);
    #[doc = "Get the `widthInMeters` field of this object."]
    #[wasm_bindgen(method, getter = "widthInMeters")]
    pub fn get_width_in_meters(this: &XrTrackedImageInit) -> f32;
    #[doc = "Change the `widthInMeters` field of this object."]
    #[wasm_bindgen(method, setter = "widthInMeters")]
    pub fn set_width_in_meters(this: &XrTrackedImageInit, val: f32);
}
impl XrTrackedImageInit {
    #[doc = "Construct a new `XrTrackedImageInit`."]
    pub fn new(image: &::wasm_bindgen::JsValue, width_in_meters: f32) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_image(image);
        ret.set_width_in_meters(width_in_meters);
        ret
    }
    #[deprecated = "Use `set_image()` instead."]
    pub fn image(&mut self, val: &::wasm_bindgen::JsValue) -> &mut Self {
        self.set_image(val);
        self
    }
    #[deprecated = "Use `set_width_in_meters()` instead."]
    pub fn width_in_meters(&mut self, val: f32) -> &mut Self {
        self.set_width_in_meters(val);
        self
    }
}
