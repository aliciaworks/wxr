#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRDepthStateInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrDepthStateInit` dictionary."]
    pub type XrDepthStateInit;
    #[doc = "Get the `dataFormatPreference` field of this object."]
    #[wasm_bindgen(method, getter = "dataFormatPreference")]
    pub fn get_data_format_preference(this: &XrDepthStateInit) -> ::js_sys::Array;
    #[doc = "Change the `dataFormatPreference` field of this object."]
    #[wasm_bindgen(method, setter = "dataFormatPreference")]
    pub fn set_data_format_preference(this: &XrDepthStateInit, val: &::wasm_bindgen::JsValue);
    #[doc = "Get the `depthTypeRequest` field of this object."]
    #[wasm_bindgen(method, getter = "depthTypeRequest")]
    pub fn get_depth_type_request(this: &XrDepthStateInit) -> Option<::js_sys::Array>;
    #[doc = "Change the `depthTypeRequest` field of this object."]
    #[wasm_bindgen(method, setter = "depthTypeRequest")]
    pub fn set_depth_type_request(this: &XrDepthStateInit, val: &::wasm_bindgen::JsValue);
    #[doc = "Get the `matchDepthView` field of this object."]
    #[wasm_bindgen(method, getter = "matchDepthView")]
    pub fn get_match_depth_view(this: &XrDepthStateInit) -> Option<bool>;
    #[doc = "Change the `matchDepthView` field of this object."]
    #[wasm_bindgen(method, setter = "matchDepthView")]
    pub fn set_match_depth_view(this: &XrDepthStateInit, val: bool);
    #[doc = "Get the `usagePreference` field of this object."]
    #[wasm_bindgen(method, getter = "usagePreference")]
    pub fn get_usage_preference(this: &XrDepthStateInit) -> ::js_sys::Array;
    #[doc = "Change the `usagePreference` field of this object."]
    #[wasm_bindgen(method, setter = "usagePreference")]
    pub fn set_usage_preference(this: &XrDepthStateInit, val: &::wasm_bindgen::JsValue);
}
impl XrDepthStateInit {
    #[doc = "Construct a new `XrDepthStateInit`."]
    pub fn new(
        data_format_preference: &::wasm_bindgen::JsValue,
        usage_preference: &::wasm_bindgen::JsValue,
    ) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_data_format_preference(data_format_preference);
        ret.set_usage_preference(usage_preference);
        ret
    }
    #[deprecated = "Use `set_data_format_preference()` instead."]
    pub fn data_format_preference(&mut self, val: &::wasm_bindgen::JsValue) -> &mut Self {
        self.set_data_format_preference(val);
        self
    }
    #[deprecated = "Use `set_depth_type_request()` instead."]
    pub fn depth_type_request(&mut self, val: &::wasm_bindgen::JsValue) -> &mut Self {
        self.set_depth_type_request(val);
        self
    }
    #[deprecated = "Use `set_match_depth_view()` instead."]
    pub fn match_depth_view(&mut self, val: bool) -> &mut Self {
        self.set_match_depth_view(val);
        self
    }
    #[deprecated = "Use `set_usage_preference()` instead."]
    pub fn usage_preference(&mut self, val: &::wasm_bindgen::JsValue) -> &mut Self {
        self.set_usage_preference(val);
        self
    }
}
