#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRVisibilityMaskChangeEventInit"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrVisibilityMaskChangeEventInit` dictionary."]
    pub type XrVisibilityMaskChangeEventInit;
    #[doc = "Get the `bubbles` field of this object."]
    #[wasm_bindgen(method, getter = "bubbles")]
    pub fn get_bubbles(this: &XrVisibilityMaskChangeEventInit) -> Option<bool>;
    #[doc = "Change the `bubbles` field of this object."]
    #[wasm_bindgen(method, setter = "bubbles")]
    pub fn set_bubbles(this: &XrVisibilityMaskChangeEventInit, val: bool);
    #[doc = "Get the `cancelable` field of this object."]
    #[wasm_bindgen(method, getter = "cancelable")]
    pub fn get_cancelable(this: &XrVisibilityMaskChangeEventInit) -> Option<bool>;
    #[doc = "Change the `cancelable` field of this object."]
    #[wasm_bindgen(method, setter = "cancelable")]
    pub fn set_cancelable(this: &XrVisibilityMaskChangeEventInit, val: bool);
    #[doc = "Get the `composed` field of this object."]
    #[wasm_bindgen(method, getter = "composed")]
    pub fn get_composed(this: &XrVisibilityMaskChangeEventInit) -> Option<bool>;
    #[doc = "Change the `composed` field of this object."]
    #[wasm_bindgen(method, setter = "composed")]
    pub fn set_composed(this: &XrVisibilityMaskChangeEventInit, val: bool);
    #[doc = "Get the `eye` field of this object."]
    #[wasm_bindgen(method, getter = "eye")]
    pub fn get_eye(this: &XrVisibilityMaskChangeEventInit) -> XrEye;
    #[doc = "Change the `eye` field of this object."]
    #[wasm_bindgen(method, setter = "eye")]
    pub fn set_eye(this: &XrVisibilityMaskChangeEventInit, val: XrEye);
    #[doc = "Get the `index` field of this object."]
    #[wasm_bindgen(method, getter = "index")]
    pub fn get_index(this: &XrVisibilityMaskChangeEventInit) -> u32;
    #[doc = "Change the `index` field of this object."]
    #[wasm_bindgen(method, setter = "index")]
    pub fn set_index(this: &XrVisibilityMaskChangeEventInit, val: u32);
    #[doc = "Get the `indices` field of this object."]
    #[wasm_bindgen(method, getter = "indices")]
    pub fn get_indices(this: &XrVisibilityMaskChangeEventInit) -> ::alloc::vec::Vec<u32>;
    #[doc = "Change the `indices` field of this object."]
    #[wasm_bindgen(method, setter = "indices")]
    pub fn set_indices(this: &XrVisibilityMaskChangeEventInit, val: &::js_sys::Uint32Array);
    #[doc = "Change the `indices` field of this object."]
    #[wasm_bindgen(method, setter = "indices")]
    pub fn set_indices_u32_slice(this: &XrVisibilityMaskChangeEventInit, val: &mut [u32]);
    #[doc = "Change the `indices` field of this object."]
    #[wasm_bindgen(method, setter = "indices")]
    pub fn set_indices_u32_array(
        this: &XrVisibilityMaskChangeEventInit,
        val: &::js_sys::Uint32Array,
    );
    #[doc = "Get the `session` field of this object."]
    #[wasm_bindgen(method, getter = "session")]
    pub fn get_session(this: &XrVisibilityMaskChangeEventInit) -> XrSession;
    #[doc = "Change the `session` field of this object."]
    #[wasm_bindgen(method, setter = "session")]
    pub fn set_session(this: &XrVisibilityMaskChangeEventInit, val: &XrSession);
    #[doc = "Get the `vertices` field of this object."]
    #[wasm_bindgen(method, getter = "vertices")]
    pub fn get_vertices(this: &XrVisibilityMaskChangeEventInit) -> ::alloc::vec::Vec<f32>;
    #[doc = "Change the `vertices` field of this object."]
    #[wasm_bindgen(method, setter = "vertices")]
    pub fn set_vertices(this: &XrVisibilityMaskChangeEventInit, val: &::js_sys::Float32Array);
    #[doc = "Change the `vertices` field of this object."]
    #[wasm_bindgen(method, setter = "vertices")]
    pub fn set_vertices_f32_slice(this: &XrVisibilityMaskChangeEventInit, val: &mut [f32]);
    #[doc = "Change the `vertices` field of this object."]
    #[wasm_bindgen(method, setter = "vertices")]
    pub fn set_vertices_f32_array(
        this: &XrVisibilityMaskChangeEventInit,
        val: &::js_sys::Float32Array,
    );
}
impl XrVisibilityMaskChangeEventInit {
    #[doc = "Construct a new `XrVisibilityMaskChangeEventInit`."]
    pub fn new(
        eye: XrEye,
        index: u32,
        indices: &::js_sys::Uint32Array,
        session: &XrSession,
        vertices: &::js_sys::Float32Array,
    ) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_eye(eye);
        ret.set_index(index);
        ret.set_indices(indices);
        ret.set_session(session);
        ret.set_vertices(vertices);
        ret
    }
    #[doc = "Construct a new `XrVisibilityMaskChangeEventInit`."]
    pub fn new_with_u32_slice(
        eye: XrEye,
        index: u32,
        indices: &mut [u32],
        session: &XrSession,
        vertices: &::js_sys::Float32Array,
    ) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_eye(eye);
        ret.set_index(index);
        ret.set_indices_u32_slice(indices);
        ret.set_session(session);
        ret.set_vertices(vertices);
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
    #[deprecated = "Use `set_eye()` instead."]
    pub fn eye(&mut self, val: XrEye) -> &mut Self {
        self.set_eye(val);
        self
    }
    #[deprecated = "Use `set_index()` instead."]
    pub fn index(&mut self, val: u32) -> &mut Self {
        self.set_index(val);
        self
    }
    #[deprecated = "Use `set_indices()` instead."]
    pub fn indices(&mut self, val: &::js_sys::Uint32Array) -> &mut Self {
        self.set_indices(val);
        self
    }
    #[deprecated = "Use `set_session()` instead."]
    pub fn session(&mut self, val: &XrSession) -> &mut Self {
        self.set_session(val);
        self
    }
    #[deprecated = "Use `set_vertices()` instead."]
    pub fn vertices(&mut self, val: &::js_sys::Float32Array) -> &mut Self {
        self.set_vertices(val);
        self
    }
}
