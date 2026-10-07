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
        js_name = "XRInputSourcesChangeEvent",
        typescript_type = "XRInputSourcesChangeEvent"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrInputSourcesChangeEvent` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourcesChangeEvent)"]
    pub type XrInputSourcesChangeEvent;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRInputSourcesChangeEvent",
        js_name = "session"
    )]
    #[doc = "Getter for the `session` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourcesChangeEvent/session)"]
    pub fn session(this: &XrInputSourcesChangeEvent) -> XrSession;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRInputSourcesChangeEvent",
        js_name = "added"
    )]
    #[doc = "Getter for the `added` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourcesChangeEvent/added)"]
    pub fn added(this: &XrInputSourcesChangeEvent) -> ::js_sys::Array;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRInputSourcesChangeEvent",
        js_name = "removed"
    )]
    #[doc = "Getter for the `removed` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourcesChangeEvent/removed)"]
    pub fn removed(this: &XrInputSourcesChangeEvent) -> ::js_sys::Array;
    #[wasm_bindgen(catch, constructor, js_class = "XRInputSourcesChangeEvent")]
    #[doc = "The `new XrInputSourcesChangeEvent(..)` constructor, creating a new instance of `XrInputSourcesChangeEvent`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourcesChangeEvent/XRInputSourcesChangeEvent)"]
    pub fn new(
        type_: &str,
        event_init_dict: &XrInputSourcesChangeEventInit,
    ) -> Result<XrInputSourcesChangeEvent, JsValue>;
}
