#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrPose",
        extends = "::js_sys::Object",
        js_name = "XRViewerPose",
        typescript_type = "XRViewerPose"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrViewerPose` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRViewerPose)"]
    pub type XrViewerPose;
    #[wasm_bindgen(method, getter, js_class = "XRViewerPose", js_name = "views")]
    #[doc = "Getter for the `views` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRViewerPose/views)"]
    pub fn views(this: &XrViewerPose) -> ::js_sys::Array;
}
