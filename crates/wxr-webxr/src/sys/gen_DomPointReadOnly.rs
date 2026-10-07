#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "DOMPointReadOnly",
        typescript_type = "DOMPointReadOnly"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `DomPointReadOnly` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/DOMPointReadOnly)"]
    pub type DomPointReadOnly;
    #[wasm_bindgen(method, getter, js_class = "DOMPointReadOnly", js_name = "x")]
    #[doc = "Getter for the `x` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/DOMPointReadOnly/x)"]
    pub fn x(this: &DomPointReadOnly) -> f64;
    #[wasm_bindgen(method, getter, js_class = "DOMPointReadOnly", js_name = "y")]
    #[doc = "Getter for the `y` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/DOMPointReadOnly/y)"]
    pub fn y(this: &DomPointReadOnly) -> f64;
    #[wasm_bindgen(method, getter, js_class = "DOMPointReadOnly", js_name = "z")]
    #[doc = "Getter for the `z` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/DOMPointReadOnly/z)"]
    pub fn z(this: &DomPointReadOnly) -> f64;
    #[wasm_bindgen(method, getter, js_class = "DOMPointReadOnly", js_name = "w")]
    #[doc = "Getter for the `w` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/DOMPointReadOnly/w)"]
    pub fn w(this: &DomPointReadOnly) -> f64;
}
