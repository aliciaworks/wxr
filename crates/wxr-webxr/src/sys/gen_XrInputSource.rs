#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "XRInputSource",
        typescript_type = "XRInputSource"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrInputSource` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSource)"]
    pub type XrInputSource;
    #[wasm_bindgen(method, getter, js_class = "XRInputSource", js_name = "gamepad")]
    #[doc = "Getter for the `gamepad` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSource/gamepad)"]
    pub fn gamepad(this: &XrInputSource) -> Option<Gamepad>;
    #[wasm_bindgen(method, getter, js_class = "XRInputSource", js_name = "hand")]
    #[doc = "Getter for the `hand` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSource/hand)"]
    pub fn hand(this: &XrInputSource) -> Option<XrHand>;
    #[wasm_bindgen(method, getter, js_class = "XRInputSource", js_name = "handedness")]
    #[doc = "Getter for the `handedness` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSource/handedness)"]
    pub fn handedness(this: &XrInputSource) -> XrHandedness;
    #[wasm_bindgen(method, getter, js_class = "XRInputSource", js_name = "targetRayMode")]
    #[doc = "Getter for the `targetRayMode` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSource/targetRayMode)"]
    pub fn target_ray_mode(this: &XrInputSource) -> XrTargetRayMode;
    #[wasm_bindgen(method, getter, js_class = "XRInputSource", js_name = "targetRaySpace")]
    #[doc = "Getter for the `targetRaySpace` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSource/targetRaySpace)"]
    pub fn target_ray_space(this: &XrInputSource) -> XrSpace;
    #[wasm_bindgen(method, getter, js_class = "XRInputSource", js_name = "gripSpace")]
    #[doc = "Getter for the `gripSpace` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSource/gripSpace)"]
    pub fn grip_space(this: &XrInputSource) -> Option<XrSpace>;
    #[wasm_bindgen(method, getter, js_class = "XRInputSource", js_name = "profiles")]
    #[doc = "Getter for the `profiles` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSource/profiles)"]
    pub fn profiles(this: &XrInputSource) -> ::js_sys::Array;
    #[wasm_bindgen(method, getter, js_class = "XRInputSource", js_name = "skipRendering")]
    #[doc = "Getter for the `skipRendering` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRInputSource/skipRendering)"]
    pub fn skip_rendering(this: &XrInputSource) -> bool;
}
