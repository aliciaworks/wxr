#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRSessionSupportedPermissionDescriptor"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrSessionSupportedPermissionDescriptor` dictionary."]
    pub type XrSessionSupportedPermissionDescriptor;
    #[doc = "Get the `name` field of this object."]
    #[wasm_bindgen(method, getter = "name")]
    pub fn get_name(this: &XrSessionSupportedPermissionDescriptor) -> ::alloc::string::String;
    #[doc = "Change the `name` field of this object."]
    #[wasm_bindgen(method, setter = "name")]
    pub fn set_name(this: &XrSessionSupportedPermissionDescriptor, val: &str);
    #[doc = "Get the `mode` field of this object."]
    #[wasm_bindgen(method, getter = "mode")]
    pub fn get_mode(this: &XrSessionSupportedPermissionDescriptor) -> Option<XrSessionMode>;
    #[doc = "Change the `mode` field of this object."]
    #[wasm_bindgen(method, setter = "mode")]
    pub fn set_mode(this: &XrSessionSupportedPermissionDescriptor, val: XrSessionMode);
}
impl XrSessionSupportedPermissionDescriptor {
    #[doc = "Construct a new `XrSessionSupportedPermissionDescriptor`."]
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
    #[deprecated = "Use `set_mode()` instead."]
    pub fn mode(&mut self, val: XrSessionMode) -> &mut Self {
        self.set_mode(val);
        self
    }
}
