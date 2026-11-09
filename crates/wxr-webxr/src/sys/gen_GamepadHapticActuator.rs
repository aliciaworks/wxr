#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "GamepadHapticActuator",
        typescript_type = "GamepadHapticActuator"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `GamepadHapticActuator` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/GamepadHapticActuator)"]
    pub type GamepadHapticActuator;
    #[wasm_bindgen(method, js_class = "GamepadHapticActuator", js_name = "playPCM")]
    #[doc = "The `playPCM()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/GamepadHapticActuator/playPCM)"]
    pub fn play_pcm(
        this: &GamepadHapticActuator,
        sound: &::wasm_bindgen::JsValue,
    ) -> ::js_sys::Promise;
    #[wasm_bindgen(method, js_class = "GamepadHapticActuator")]
    #[doc = "The `pulse()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/GamepadHapticActuator/pulse)"]
    pub fn pulse(this: &GamepadHapticActuator, value: f64, duration: f64) -> ::js_sys::Promise;
}
