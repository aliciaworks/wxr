#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRInputSourcesChangeEventInit"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrInputSourcesChangeEventInit` dictionary."]
    pub type XrInputSourcesChangeEventInit;
    #[doc = "Get the `bubbles` field of this object."]
    #[wasm_bindgen(method, getter = "bubbles")]
    pub fn get_bubbles(this: &XrInputSourcesChangeEventInit) -> Option<bool>;
    #[doc = "Change the `bubbles` field of this object."]
    #[wasm_bindgen(method, setter = "bubbles")]
    pub fn set_bubbles(this: &XrInputSourcesChangeEventInit, val: bool);
    #[doc = "Get the `cancelable` field of this object."]
    #[wasm_bindgen(method, getter = "cancelable")]
    pub fn get_cancelable(this: &XrInputSourcesChangeEventInit) -> Option<bool>;
    #[doc = "Change the `cancelable` field of this object."]
    #[wasm_bindgen(method, setter = "cancelable")]
    pub fn set_cancelable(this: &XrInputSourcesChangeEventInit, val: bool);
    #[doc = "Get the `composed` field of this object."]
    #[wasm_bindgen(method, getter = "composed")]
    pub fn get_composed(this: &XrInputSourcesChangeEventInit) -> Option<bool>;
    #[doc = "Change the `composed` field of this object."]
    #[wasm_bindgen(method, setter = "composed")]
    pub fn set_composed(this: &XrInputSourcesChangeEventInit, val: bool);
    #[doc = "Get the `added` field of this object."]
    #[wasm_bindgen(method, getter = "added")]
    pub fn get_added(this: &XrInputSourcesChangeEventInit) -> ::js_sys::Array;
    #[doc = "Change the `added` field of this object."]
    #[wasm_bindgen(method, setter = "added")]
    pub fn set_added(this: &XrInputSourcesChangeEventInit, val: &::wasm_bindgen::JsValue);
    #[doc = "Get the `removed` field of this object."]
    #[wasm_bindgen(method, getter = "removed")]
    pub fn get_removed(this: &XrInputSourcesChangeEventInit) -> ::js_sys::Array;
    #[doc = "Change the `removed` field of this object."]
    #[wasm_bindgen(method, setter = "removed")]
    pub fn set_removed(this: &XrInputSourcesChangeEventInit, val: &::wasm_bindgen::JsValue);
    #[doc = "Get the `session` field of this object."]
    #[wasm_bindgen(method, getter = "session")]
    pub fn get_session(this: &XrInputSourcesChangeEventInit) -> XrSession;
    #[doc = "Change the `session` field of this object."]
    #[wasm_bindgen(method, setter = "session")]
    pub fn set_session(this: &XrInputSourcesChangeEventInit, val: &XrSession);
}
impl XrInputSourcesChangeEventInit {
    #[doc = "Construct a new `XrInputSourcesChangeEventInit`."]
    pub fn new(
        added: &::wasm_bindgen::JsValue,
        removed: &::wasm_bindgen::JsValue,
        session: &XrSession,
    ) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_added(added);
        ret.set_removed(removed);
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
    #[deprecated = "Use `set_added()` instead."]
    pub fn added(&mut self, val: &::wasm_bindgen::JsValue) -> &mut Self {
        self.set_added(val);
        self
    }
    #[deprecated = "Use `set_removed()` instead."]
    pub fn removed(&mut self, val: &::wasm_bindgen::JsValue) -> &mut Self {
        self.set_removed(val);
        self
    }
    #[deprecated = "Use `set_session()` instead."]
    pub fn session(&mut self, val: &XrSession) -> &mut Self {
        self.set_session(val);
        self
    }
}
