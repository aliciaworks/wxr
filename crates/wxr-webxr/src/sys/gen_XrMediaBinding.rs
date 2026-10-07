#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRMediaBinding",
        typescript_type = "XRMediaBinding"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrMediaBinding` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMediaBinding)"]
    pub type XrMediaBinding;
    #[wasm_bindgen(catch, constructor, js_class = "XRMediaBinding")]
    #[doc = "The `new XrMediaBinding(..)` constructor, creating a new instance of `XrMediaBinding`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRMediaBinding/XRMediaBinding)"]
    pub fn new(session: &XrSession) -> Result<XrMediaBinding, JsValue>;
}
