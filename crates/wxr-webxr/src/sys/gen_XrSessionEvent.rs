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
        js_name = "XRSessionEvent",
        typescript_type = "XRSessionEvent"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrSessionEvent` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSessionEvent)"]
    pub type XrSessionEvent;
    #[wasm_bindgen(method, getter, js_class = "XRSessionEvent", js_name = "session")]
    #[doc = "Getter for the `session` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSessionEvent/session)"]
    pub fn session(this: &XrSessionEvent) -> XrSession;
    #[wasm_bindgen(catch, constructor, js_class = "XRSessionEvent")]
    #[doc = "The `new XrSessionEvent(..)` constructor, creating a new instance of `XrSessionEvent`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSessionEvent/XRSessionEvent)"]
    pub fn new(
        type_: &str,
        event_init_dict: &XrSessionEventInit,
    ) -> Result<XrSessionEvent, JsValue>;
}
