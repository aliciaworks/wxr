#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRDOMOverlayState")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrdomOverlayState` dictionary."]
    pub type XrdomOverlayState;
    #[doc = "Get the `type` field of this object."]
    #[wasm_bindgen(method, getter = "type")]
    pub fn get_type(this: &XrdomOverlayState) -> Option<XrdomOverlayType>;
    #[doc = "Change the `type` field of this object."]
    #[wasm_bindgen(method, setter = "type")]
    pub fn set_type(this: &XrdomOverlayState, val: XrdomOverlayType);
}
impl XrdomOverlayState {
    #[doc = "Construct a new `XrdomOverlayState`."]
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret
    }
    #[deprecated = "Use `set_type()` instead."]
    pub fn type_(&mut self, val: XrdomOverlayType) -> &mut Self {
        self.set_type(val);
        self
    }
}
impl Default for XrdomOverlayState {
    fn default() -> Self {
        Self::new()
    }
}
