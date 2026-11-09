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
        js_name = "XRReferenceSpaceEvent",
        typescript_type = "XRReferenceSpaceEvent"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrReferenceSpaceEvent` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRReferenceSpaceEvent)"]
    pub type XrReferenceSpaceEvent;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRReferenceSpaceEvent",
        js_name = "referenceSpace"
    )]
    #[doc = "Getter for the `referenceSpace` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRReferenceSpaceEvent/referenceSpace)"]
    pub fn reference_space(this: &XrReferenceSpaceEvent) -> XrReferenceSpace;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRReferenceSpaceEvent",
        js_name = "transform"
    )]
    #[doc = "Getter for the `transform` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRReferenceSpaceEvent/transform)"]
    pub fn transform(this: &XrReferenceSpaceEvent) -> Option<XrRigidTransform>;
    #[wasm_bindgen(catch, constructor, js_class = "XRReferenceSpaceEvent")]
    #[doc = "The `new XrReferenceSpaceEvent(..)` constructor, creating a new instance of `XrReferenceSpaceEvent`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRReferenceSpaceEvent/XRReferenceSpaceEvent)"]
    pub fn new(
        type_: &str,
        event_init_dict: &XrReferenceSpaceEventInit,
    ) -> Result<XrReferenceSpaceEvent, JsValue>;
}
