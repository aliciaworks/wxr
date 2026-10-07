#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRMediaLayerInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrMediaLayerInit` dictionary."]
    pub type XrMediaLayerInit;
    #[doc = "Get the `invertStereo` field of this object."]
    #[wasm_bindgen(method, getter = "invertStereo")]
    pub fn get_invert_stereo(this: &XrMediaLayerInit) -> Option<bool>;
    #[doc = "Change the `invertStereo` field of this object."]
    #[wasm_bindgen(method, setter = "invertStereo")]
    pub fn set_invert_stereo(this: &XrMediaLayerInit, val: bool);
    #[doc = "Get the `layout` field of this object."]
    #[wasm_bindgen(method, getter = "layout")]
    pub fn get_layout(this: &XrMediaLayerInit) -> Option<XrLayerLayout>;
    #[doc = "Change the `layout` field of this object."]
    #[wasm_bindgen(method, setter = "layout")]
    pub fn set_layout(this: &XrMediaLayerInit, val: XrLayerLayout);
    #[doc = "Get the `space` field of this object."]
    #[wasm_bindgen(method, getter = "space")]
    pub fn get_space(this: &XrMediaLayerInit) -> XrSpace;
    #[doc = "Change the `space` field of this object."]
    #[wasm_bindgen(method, setter = "space")]
    pub fn set_space(this: &XrMediaLayerInit, val: &XrSpace);
}
impl XrMediaLayerInit {
    #[doc = "Construct a new `XrMediaLayerInit`."]
    pub fn new(space: &XrSpace) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_space(space);
        ret
    }
    #[deprecated = "Use `set_invert_stereo()` instead."]
    pub fn invert_stereo(&mut self, val: bool) -> &mut Self {
        self.set_invert_stereo(val);
        self
    }
    #[deprecated = "Use `set_layout()` instead."]
    pub fn layout(&mut self, val: XrLayerLayout) -> &mut Self {
        self.set_layout(val);
        self
    }
    #[deprecated = "Use `set_space()` instead."]
    pub fn space(&mut self, val: &XrSpace) -> &mut Self {
        self.set_space(val);
        self
    }
}
