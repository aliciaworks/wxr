#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRInputSourceEventInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrInputSourceEventInit` dictionary."]
    pub type XrInputSourceEventInit;
    #[doc = "Get the `bubbles` field of this object."]
    #[wasm_bindgen(method, getter = "bubbles")]
    pub fn get_bubbles(this: &XrInputSourceEventInit) -> Option<bool>;
    #[doc = "Change the `bubbles` field of this object."]
    #[wasm_bindgen(method, setter = "bubbles")]
    pub fn set_bubbles(this: &XrInputSourceEventInit, val: bool);
    #[doc = "Get the `cancelable` field of this object."]
    #[wasm_bindgen(method, getter = "cancelable")]
    pub fn get_cancelable(this: &XrInputSourceEventInit) -> Option<bool>;
    #[doc = "Change the `cancelable` field of this object."]
    #[wasm_bindgen(method, setter = "cancelable")]
    pub fn set_cancelable(this: &XrInputSourceEventInit, val: bool);
    #[doc = "Get the `composed` field of this object."]
    #[wasm_bindgen(method, getter = "composed")]
    pub fn get_composed(this: &XrInputSourceEventInit) -> Option<bool>;
    #[doc = "Change the `composed` field of this object."]
    #[wasm_bindgen(method, setter = "composed")]
    pub fn set_composed(this: &XrInputSourceEventInit, val: bool);
    #[doc = "Get the `frame` field of this object."]
    #[wasm_bindgen(method, getter = "frame")]
    pub fn get_frame(this: &XrInputSourceEventInit) -> XrFrame;
    #[doc = "Change the `frame` field of this object."]
    #[wasm_bindgen(method, setter = "frame")]
    pub fn set_frame(this: &XrInputSourceEventInit, val: &XrFrame);
    #[doc = "Get the `inputSource` field of this object."]
    #[wasm_bindgen(method, getter = "inputSource")]
    pub fn get_input_source(this: &XrInputSourceEventInit) -> XrInputSource;
    #[doc = "Change the `inputSource` field of this object."]
    #[wasm_bindgen(method, setter = "inputSource")]
    pub fn set_input_source(this: &XrInputSourceEventInit, val: &XrInputSource);
}
impl XrInputSourceEventInit {
    #[doc = "Construct a new `XrInputSourceEventInit`."]
    pub fn new(frame: &XrFrame, input_source: &XrInputSource) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_frame(frame);
        ret.set_input_source(input_source);
        ret
    }
    #[deprecated = "Use `set_bubbles()` instead."]
    pub fn bubbles(&mut self, val: bool) -> &mut Self {
        self.set_bubbles(val);
        self
    }
    #[deprecated = "Use `set_cancelable()` instead."]
    pub fn cancelable(&mut self, val: bool) -> &mut Self {
        self.set_cancelable(val);
        self
    }
    #[deprecated = "Use `set_composed()` instead."]
    pub fn composed(&mut self, val: bool) -> &mut Self {
        self.set_composed(val);
        self
    }
    #[deprecated = "Use `set_frame()` instead."]
    pub fn frame(&mut self, val: &XrFrame) -> &mut Self {
        self.set_frame(val);
        self
    }
    #[deprecated = "Use `set_input_source()` instead."]
    pub fn input_source(&mut self, val: &XrInputSource) -> &mut Self {
        self.set_input_source(val);
        self
    }
}
