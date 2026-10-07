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
        js_name = "XRLayerEvent",
        typescript_type = "XRLayerEvent"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrLayerEvent` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRLayerEvent)"]
    pub type XrLayerEvent;
    #[wasm_bindgen(method, getter, js_class = "XRLayerEvent", js_name = "layer")]
    #[doc = "Getter for the `layer` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRLayerEvent/layer)"]
    pub fn layer(this: &XrLayerEvent) -> XrLayer;
    #[wasm_bindgen(catch, constructor, js_class = "XRLayerEvent")]
    #[doc = "The `new XrLayerEvent(..)` constructor, creating a new instance of `XrLayerEvent`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRLayerEvent/XRLayerEvent)"]
    pub fn new(type_: &str, event_init_dict: &XrLayerEventInit) -> Result<XrLayerEvent, JsValue>;
}
