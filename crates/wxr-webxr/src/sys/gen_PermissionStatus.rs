#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "PermissionStatus",
        typescript_type = "PermissionStatus"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `PermissionStatus` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/PermissionStatus)"]
    pub type PermissionStatus;
}
