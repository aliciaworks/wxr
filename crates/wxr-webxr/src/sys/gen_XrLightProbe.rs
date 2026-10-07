#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "XRLightProbe",
        typescript_type = "XRLightProbe"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrLightProbe` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRLightProbe)"]
    pub type XrLightProbe;
    #[wasm_bindgen(method, getter, js_class = "XRLightProbe", js_name = "probeSpace")]
    #[doc = "Getter for the `probeSpace` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRLightProbe/probeSpace)"]
    pub fn probe_space(this: &XrLightProbe) -> XrSpace;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRLightProbe",
        js_name = "onreflectionchange"
    )]
    #[doc = "Getter for the `onreflectionchange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRLightProbe/onreflectionchange)"]
    pub fn onreflectionchange(this: &XrLightProbe) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(
        method,
        setter,
        js_class = "XRLightProbe",
        js_name = "onreflectionchange"
    )]
    #[doc = "Setter for the `onreflectionchange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRLightProbe/onreflectionchange)"]
    pub fn set_onreflectionchange(this: &XrLightProbe, value: &::wasm_bindgen::JsValue);
}
