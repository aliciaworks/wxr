#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "Event",
        typescript_type = "Event"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `Event` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Event)"]
    pub type Event;
}
