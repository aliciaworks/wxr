#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "EventTarget",
        typescript_type = "EventTarget"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `EventTarget` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget)"]
    pub type EventTarget;
    #[wasm_bindgen(method, js_class = "EventTarget", js_name = "addEventListener")]
    #[doc = "The `addEventListener()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/addEventListener)"]
    pub fn add_event_listener_with_opt_callback(
        this: &EventTarget,
        type_: &str,
        callback: Option<&::js_sys::Function>,
    );
    #[wasm_bindgen(method, js_class = "EventTarget", js_name = "addEventListener")]
    #[doc = "The `addEventListener()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/addEventListener)"]
    pub fn add_event_listener_with_opt_event_listener(
        this: &EventTarget,
        type_: &str,
        callback: Option<&EventListener>,
    );
    #[wasm_bindgen(method, js_class = "EventTarget", js_name = "dispatchEvent")]
    #[doc = "The `dispatchEvent()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/dispatchEvent)"]
    pub fn dispatch_event(this: &EventTarget, event: &Event) -> bool;
    #[wasm_bindgen(method, js_class = "EventTarget", js_name = "removeEventListener")]
    #[doc = "The `removeEventListener()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/removeEventListener)"]
    pub fn remove_event_listener_with_opt_callback(
        this: &EventTarget,
        type_: &str,
        callback: Option<&::js_sys::Function>,
    );
    #[wasm_bindgen(method, js_class = "EventTarget", js_name = "removeEventListener")]
    #[doc = "The `removeEventListener()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/removeEventListener)"]
    pub fn remove_event_listener_with_opt_event_listener(
        this: &EventTarget,
        type_: &str,
        callback: Option<&EventListener>,
    );
}
