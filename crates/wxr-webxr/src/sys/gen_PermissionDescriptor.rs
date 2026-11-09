#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "PermissionDescriptor")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `PermissionDescriptor` dictionary."]
    pub type PermissionDescriptor;
    #[doc = "Get the `name` field of this object."]
    #[wasm_bindgen(method, getter = "name")]
    pub fn get_name(this: &PermissionDescriptor) -> ::alloc::string::String;
    #[doc = "Change the `name` field of this object."]
    #[wasm_bindgen(method, setter = "name")]
    pub fn set_name(this: &PermissionDescriptor, val: &str);
}
impl PermissionDescriptor {
    #[doc = "Construct a new `PermissionDescriptor`."]
    pub fn new(name: &str) -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret.set_name(name);
        ret
    }
    #[deprecated = "Use `set_name()` instead."]
    pub fn name(&mut self, val: &str) -> &mut Self {
        self.set_name(val);
        self
    }
}
