#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "PermissionStatus",
        extends = "::js_sys::Object",
        js_name = "XRPermissionStatus",
        typescript_type = "XRPermissionStatus"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrPermissionStatus` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPermissionStatus)"]
    pub type XrPermissionStatus;
    #[wasm_bindgen(method, getter, js_class = "XRPermissionStatus", js_name = "granted")]
    #[doc = "Getter for the `granted` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPermissionStatus/granted)"]
    pub fn granted(this: &XrPermissionStatus) -> ::js_sys::Array;
    #[wasm_bindgen(method, setter, js_class = "XRPermissionStatus", js_name = "granted")]
    #[doc = "Setter for the `granted` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRPermissionStatus/granted)"]
    pub fn set_granted(this: &XrPermissionStatus, value: &::wasm_bindgen::JsValue);
}
