#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "GPURequestAdapterOptions")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `GpuRequestAdapterOptions` dictionary."]
    pub type GpuRequestAdapterOptions;
    #[doc = "Get the `xrCompatible` field of this object."]
    #[wasm_bindgen(method, getter = "xrCompatible")]
    pub fn get_xr_compatible(this: &GpuRequestAdapterOptions) -> Option<bool>;
    #[doc = "Change the `xrCompatible` field of this object."]
    #[wasm_bindgen(method, setter = "xrCompatible")]
    pub fn set_xr_compatible(this: &GpuRequestAdapterOptions, val: bool);
}
impl GpuRequestAdapterOptions {
    #[doc = "Construct a new `GpuRequestAdapterOptions`."]
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret
    }
    #[deprecated = "Use `set_xr_compatible()` instead."]
    pub fn xr_compatible(&mut self, val: bool) -> &mut Self {
        self.set_xr_compatible(val);
        self
    }
}
impl Default for GpuRequestAdapterOptions {
    fn default() -> Self {
        Self::new()
    }
}
