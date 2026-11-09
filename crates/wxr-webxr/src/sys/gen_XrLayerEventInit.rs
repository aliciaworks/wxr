#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRLayerEventInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrLayerEventInit` dictionary."]
    pub type XrLayerEventInit;
    #[doc = "Get the `bubbles` field of this object."]
    #[wasm_bindgen(method, getter = "bubbles")]
    pub fn get_bubbles(this: &XrLayerEventInit) -> Option<bool>;
    #[doc = "Change the `bubbles` field of this object."]
    #[wasm_bindgen(method, setter = "bubbles")]
    pub fn set_bubbles(this: &XrLayerEventInit, val: bool);
    #[doc = "Get the `cancelable` field of this object."]
    #[wasm_bindgen(method, getter = "cancelable")]
    pub fn get_cancelable(this: &XrLayerEventInit) -> Option<bool>;
    #[doc = "Change the `cancelable` field of this object."]
    #[wasm_bindgen(method, setter = "cancelable")]
    pub fn set_cancelable(this: &XrLayerEventInit, val: bool);
    #[doc = "Get the `composed` field of this object."]
    #[wasm_bindgen(method, getter = "composed")]
    pub fn get_composed(this: &XrLayerEventInit) -> Option<bool>;
    #[doc = "Change the `composed` field of this object."]
    #[wasm_bindgen(method, setter = "composed")]
    pub fn set_composed(this: &XrLayerEventInit, val: bool);
    #[doc = "Get the `layer` field of this object."]
    #[wasm_bindgen(method, getter = "layer")]
    pub fn get_layer(this: &XrLayerEventInit) -> XrLayer;
    #[doc = "Change the `layer` field of this object."]
    #[wasm_bindgen(method, setter = "layer")]
    pub fn set_layer(this: &XrLayerEventInit, val: &XrLayer);
}
impl XrLayerEventInit {
    #[doc = "Construct a new `XrLayerEventInit`."]
    pub fn new(layer: &XrLayer) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_layer(layer);
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
    #[deprecated = "Use `set_layer()` instead."]
    pub fn layer(&mut self, val: &XrLayer) -> &mut Self {
        self.set_layer(val);
        self
    }
}
