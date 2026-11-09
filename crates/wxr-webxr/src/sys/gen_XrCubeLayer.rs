#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "XrCompositionLayer",
        extends = "XrLayer",
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "XRCubeLayer",
        typescript_type = "XRCubeLayer"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrCubeLayer` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCubeLayer)"]
    pub type XrCubeLayer;
    #[wasm_bindgen(method, getter, js_class = "XRCubeLayer", js_name = "space")]
    #[doc = "Getter for the `space` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCubeLayer/space)"]
    pub fn space(this: &XrCubeLayer) -> XrSpace;
    #[wasm_bindgen(method, setter, js_class = "XRCubeLayer", js_name = "space")]
    #[doc = "Setter for the `space` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCubeLayer/space)"]
    pub fn set_space(this: &XrCubeLayer, value: &XrSpace);
    #[wasm_bindgen(method, getter, js_class = "XRCubeLayer", js_name = "orientation")]
    #[doc = "Getter for the `orientation` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCubeLayer/orientation)"]
    pub fn orientation(this: &XrCubeLayer) -> DomPointReadOnly;
    #[wasm_bindgen(method, setter, js_class = "XRCubeLayer", js_name = "orientation")]
    #[doc = "Setter for the `orientation` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCubeLayer/orientation)"]
    pub fn set_orientation(this: &XrCubeLayer, value: &DomPointReadOnly);
    #[wasm_bindgen(method, getter, js_class = "XRCubeLayer", js_name = "onredraw")]
    #[doc = "Getter for the `onredraw` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCubeLayer/onredraw)"]
    pub fn onredraw(this: &XrCubeLayer) -> Option<::js_sys::Function>;
    #[wasm_bindgen(method, setter, js_class = "XRCubeLayer", js_name = "onredraw")]
    #[doc = "Setter for the `onredraw` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRCubeLayer/onredraw)"]
    pub fn set_onredraw(this: &XrCubeLayer, value: Option<&::js_sys::Function>);
}
