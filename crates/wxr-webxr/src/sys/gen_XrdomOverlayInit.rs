#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRDOMOverlayInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrdomOverlayInit` dictionary."]
    pub type XrdomOverlayInit;
    #[doc = "Get the `root` field of this object."]
    #[wasm_bindgen(method, getter = "root")]
    pub fn get_root(this: &XrdomOverlayInit) -> ::wasm_bindgen::JsValue;
    #[doc = "Change the `root` field of this object."]
    #[wasm_bindgen(method, setter = "root")]
    pub fn set_root(this: &XrdomOverlayInit, val: &::wasm_bindgen::JsValue);
}
impl XrdomOverlayInit {
    #[doc = "Construct a new `XrdomOverlayInit`."]
    pub fn new(root: &::wasm_bindgen::JsValue) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_root(root);
        ret
    }
    #[deprecated = "Use `set_root()` instead."]
    pub fn root(&mut self, val: &::wasm_bindgen::JsValue) -> &mut Self {
        self.set_root(val);
        self
    }
}
