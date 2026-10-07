#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "XRSystem",
        typescript_type = "XRSystem"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrSystem` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSystem)"]
    pub type XrSystem;
    #[wasm_bindgen(method, getter, js_class = "XRSystem", js_name = "ondevicechange")]
    #[doc = "Getter for the `ondevicechange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSystem/ondevicechange)"]
    pub fn ondevicechange(this: &XrSystem) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, setter, js_class = "XRSystem", js_name = "ondevicechange")]
    #[doc = "Setter for the `ondevicechange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSystem/ondevicechange)"]
    pub fn set_ondevicechange(this: &XrSystem, value: &::wasm_bindgen::JsValue);
    #[wasm_bindgen(method, js_class = "XRSystem", js_name = "isSessionSupported")]
    #[doc = "The `isSessionSupported()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSystem/isSessionSupported)"]
    pub fn is_session_supported(this: &XrSystem, mode: XrSessionMode) -> ::js_sys::Promise;
    #[wasm_bindgen(method, js_class = "XRSystem", js_name = "requestSession")]
    #[doc = "The `requestSession()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSystem/requestSession)"]
    pub fn request_session(this: &XrSystem, mode: XrSessionMode) -> ::js_sys::Promise;
    #[wasm_bindgen(method, js_class = "XRSystem", js_name = "requestSession")]
    #[doc = "The `requestSession()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSystem/requestSession)"]
    pub fn request_session_with_options(
        this: &XrSystem,
        mode: XrSessionMode,
        options: &XrSessionInit,
    ) -> ::js_sys::Promise;
}
