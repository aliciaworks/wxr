#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRSessionEventInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrSessionEventInit` dictionary."]
    pub type XrSessionEventInit;
    #[doc = "Get the `bubbles` field of this object."]
    #[wasm_bindgen(method, getter = "bubbles")]
    pub fn get_bubbles(this: &XrSessionEventInit) -> Option<bool>;
    #[doc = "Change the `bubbles` field of this object."]
    #[wasm_bindgen(method, setter = "bubbles")]
    pub fn set_bubbles(this: &XrSessionEventInit, val: bool);
    #[doc = "Get the `cancelable` field of this object."]
    #[wasm_bindgen(method, getter = "cancelable")]
    pub fn get_cancelable(this: &XrSessionEventInit) -> Option<bool>;
    #[doc = "Change the `cancelable` field of this object."]
    #[wasm_bindgen(method, setter = "cancelable")]
    pub fn set_cancelable(this: &XrSessionEventInit, val: bool);
    #[doc = "Get the `composed` field of this object."]
    #[wasm_bindgen(method, getter = "composed")]
    pub fn get_composed(this: &XrSessionEventInit) -> Option<bool>;
    #[doc = "Change the `composed` field of this object."]
    #[wasm_bindgen(method, setter = "composed")]
    pub fn set_composed(this: &XrSessionEventInit, val: bool);
    #[doc = "Get the `session` field of this object."]
    #[wasm_bindgen(method, getter = "session")]
    pub fn get_session(this: &XrSessionEventInit) -> XrSession;
    #[doc = "Change the `session` field of this object."]
    #[wasm_bindgen(method, setter = "session")]
    pub fn set_session(this: &XrSessionEventInit, val: &XrSession);
}
impl XrSessionEventInit {
    #[doc = "Construct a new `XrSessionEventInit`."]
    pub fn new(session: &XrSession) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_session(session);
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
    #[deprecated = "Use `set_session()` instead."]
    pub fn session(&mut self, val: &XrSession) -> &mut Self {
        self.set_session(val);
        self
    }
}
