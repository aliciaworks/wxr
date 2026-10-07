#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrSpace",
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "XRReferenceSpace",
        typescript_type = "XRReferenceSpace"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrReferenceSpace` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRReferenceSpace)"]
    pub type XrReferenceSpace;
    #[wasm_bindgen(method, getter, js_class = "XRReferenceSpace", js_name = "onreset")]
    #[doc = "Getter for the `onreset` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRReferenceSpace/onreset)"]
    pub fn onreset(this: &XrReferenceSpace) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, setter, js_class = "XRReferenceSpace", js_name = "onreset")]
    #[doc = "Setter for the `onreset` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRReferenceSpace/onreset)"]
    pub fn set_onreset(this: &XrReferenceSpace, value: &::wasm_bindgen::JsValue);
    #[wasm_bindgen(
        method,
        js_class = "XRReferenceSpace",
        js_name = "getOffsetReferenceSpace"
    )]
    #[doc = "The `getOffsetReferenceSpace()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRReferenceSpace/getOffsetReferenceSpace)"]
    pub fn get_offset_reference_space(
        this: &XrReferenceSpace,
        origin_offset: &XrRigidTransform,
    ) -> XrReferenceSpace;
}
