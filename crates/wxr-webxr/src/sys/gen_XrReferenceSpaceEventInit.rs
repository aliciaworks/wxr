#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRReferenceSpaceEventInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrReferenceSpaceEventInit` dictionary."]
    pub type XrReferenceSpaceEventInit;
    #[doc = "Get the `bubbles` field of this object."]
    #[wasm_bindgen(method, getter = "bubbles")]
    pub fn get_bubbles(this: &XrReferenceSpaceEventInit) -> Option<bool>;
    #[doc = "Change the `bubbles` field of this object."]
    #[wasm_bindgen(method, setter = "bubbles")]
    pub fn set_bubbles(this: &XrReferenceSpaceEventInit, val: bool);
    #[doc = "Get the `cancelable` field of this object."]
    #[wasm_bindgen(method, getter = "cancelable")]
    pub fn get_cancelable(this: &XrReferenceSpaceEventInit) -> Option<bool>;
    #[doc = "Change the `cancelable` field of this object."]
    #[wasm_bindgen(method, setter = "cancelable")]
    pub fn set_cancelable(this: &XrReferenceSpaceEventInit, val: bool);
    #[doc = "Get the `composed` field of this object."]
    #[wasm_bindgen(method, getter = "composed")]
    pub fn get_composed(this: &XrReferenceSpaceEventInit) -> Option<bool>;
    #[doc = "Change the `composed` field of this object."]
    #[wasm_bindgen(method, setter = "composed")]
    pub fn set_composed(this: &XrReferenceSpaceEventInit, val: bool);
    #[doc = "Get the `referenceSpace` field of this object."]
    #[wasm_bindgen(method, getter = "referenceSpace")]
    pub fn get_reference_space(this: &XrReferenceSpaceEventInit) -> XrReferenceSpace;
    #[doc = "Change the `referenceSpace` field of this object."]
    #[wasm_bindgen(method, setter = "referenceSpace")]
    pub fn set_reference_space(this: &XrReferenceSpaceEventInit, val: &XrReferenceSpace);
    #[doc = "Get the `transform` field of this object."]
    #[wasm_bindgen(method, getter = "transform")]
    pub fn get_transform(this: &XrReferenceSpaceEventInit) -> Option<XrRigidTransform>;
    #[doc = "Change the `transform` field of this object."]
    #[wasm_bindgen(method, setter = "transform")]
    pub fn set_transform(this: &XrReferenceSpaceEventInit, val: Option<&XrRigidTransform>);
}
impl XrReferenceSpaceEventInit {
    #[doc = "Construct a new `XrReferenceSpaceEventInit`."]
    pub fn new(reference_space: &XrReferenceSpace) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_reference_space(reference_space);
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
    #[deprecated = "Use `set_reference_space()` instead."]
    pub fn reference_space(&mut self, val: &XrReferenceSpace) -> &mut Self {
        self.set_reference_space(val);
        self
    }
    #[deprecated = "Use `set_transform()` instead."]
    pub fn transform(&mut self, val: Option<&XrRigidTransform>) -> &mut Self {
        self.set_transform(val);
        self
    }
}
