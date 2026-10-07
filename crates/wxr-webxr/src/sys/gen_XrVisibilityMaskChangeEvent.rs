#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "Event",
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "XRVisibilityMaskChangeEvent",
        typescript_type = "XRVisibilityMaskChangeEvent"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrVisibilityMaskChangeEvent` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRVisibilityMaskChangeEvent)"]
    pub type XrVisibilityMaskChangeEvent;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRVisibilityMaskChangeEvent",
        js_name = "session"
    )]
    #[doc = "Getter for the `session` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRVisibilityMaskChangeEvent/session)"]
    pub fn session(this: &XrVisibilityMaskChangeEvent) -> XrSession;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRVisibilityMaskChangeEvent",
        js_name = "eye"
    )]
    #[doc = "Getter for the `eye` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRVisibilityMaskChangeEvent/eye)"]
    pub fn eye(this: &XrVisibilityMaskChangeEvent) -> XrEye;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRVisibilityMaskChangeEvent",
        js_name = "index"
    )]
    #[doc = "Getter for the `index` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRVisibilityMaskChangeEvent/index)"]
    pub fn index(this: &XrVisibilityMaskChangeEvent) -> u32;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRVisibilityMaskChangeEvent",
        js_name = "vertices"
    )]
    #[doc = "Getter for the `vertices` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRVisibilityMaskChangeEvent/vertices)"]
    pub fn vertices(this: &XrVisibilityMaskChangeEvent) -> ::alloc::vec::Vec<f32>;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRVisibilityMaskChangeEvent",
        js_name = "indices"
    )]
    #[doc = "Getter for the `indices` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRVisibilityMaskChangeEvent/indices)"]
    pub fn indices(this: &XrVisibilityMaskChangeEvent) -> ::alloc::vec::Vec<u32>;
    #[wasm_bindgen(catch, constructor, js_class = "XRVisibilityMaskChangeEvent")]
    #[doc = "The `new XrVisibilityMaskChangeEvent(..)` constructor, creating a new instance of `XrVisibilityMaskChangeEvent`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRVisibilityMaskChangeEvent/XRVisibilityMaskChangeEvent)"]
    pub fn new(
        type_: &str,
        event_init_dict: &XrVisibilityMaskChangeEventInit,
    ) -> Result<XrVisibilityMaskChangeEvent, JsValue>;
}
