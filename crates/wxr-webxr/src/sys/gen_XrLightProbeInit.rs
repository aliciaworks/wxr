#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "XRLightProbeInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrLightProbeInit` dictionary."]
    pub type XrLightProbeInit;
    #[doc = "Get the `reflectionFormat` field of this object."]
    #[wasm_bindgen(method, getter = "reflectionFormat")]
    pub fn get_reflection_format(this: &XrLightProbeInit) -> Option<XrReflectionFormat>;
    #[doc = "Change the `reflectionFormat` field of this object."]
    #[wasm_bindgen(method, setter = "reflectionFormat")]
    pub fn set_reflection_format(this: &XrLightProbeInit, val: XrReflectionFormat);
}
impl XrLightProbeInit {
    #[doc = "Construct a new `XrLightProbeInit`."]
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret
    }
    #[deprecated = "Use `set_reflection_format()` instead."]
    pub fn reflection_format(&mut self, val: XrReflectionFormat) -> &mut Self {
        self.set_reflection_format(val);
        self
    }
}
impl Default for XrLightProbeInit {
    fn default() -> Self {
        Self::new()
    }
}
