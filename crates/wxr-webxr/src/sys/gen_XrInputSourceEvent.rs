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
        js_name = "XRInputSourceEvent",
        typescript_type = "XRInputSourceEvent"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrInputSourceEvent` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourceEvent)"]
    pub type XrInputSourceEvent;
    #[wasm_bindgen(method, getter, js_class = "XRInputSourceEvent", js_name = "frame")]
    #[doc = "Getter for the `frame` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourceEvent/frame)"]
    pub fn frame(this: &XrInputSourceEvent) -> XrFrame;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRInputSourceEvent",
        js_name = "inputSource"
    )]
    #[doc = "Getter for the `inputSource` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourceEvent/inputSource)"]
    pub fn input_source(this: &XrInputSourceEvent) -> XrInputSource;
    #[wasm_bindgen(catch, constructor, js_class = "XRInputSourceEvent")]
    #[doc = "The `new XrInputSourceEvent(..)` constructor, creating a new instance of `XrInputSourceEvent`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSourceEvent/XRInputSourceEvent)"]
    pub fn new(
        type_: &str,
        event_init_dict: &XrInputSourceEventInit,
    ) -> Result<XrInputSourceEvent, JsValue>;
}
