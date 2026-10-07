#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "XRSession",
        typescript_type = "XRSession"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `XrSession` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession)"]
    pub type XrSession;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRSession",
        js_name = "environmentBlendMode"
    )]
    #[doc = "Getter for the `environmentBlendMode` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/environmentBlendMode)"]
    pub fn environment_blend_mode(this: &XrSession) -> XrEnvironmentBlendMode;
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "interactionMode")]
    #[doc = "Getter for the `interactionMode` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/interactionMode)"]
    pub fn interaction_mode(this: &XrSession) -> XrInteractionMode;
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "depthUsage")]
    #[doc = "Getter for the `depthUsage` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/depthUsage)"]
    pub fn depth_usage(this: &XrSession) -> XrDepthUsage;
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "depthDataFormat")]
    #[doc = "Getter for the `depthDataFormat` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/depthDataFormat)"]
    pub fn depth_data_format(this: &XrSession) -> XrDepthDataFormat;
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "depthType")]
    #[doc = "Getter for the `depthType` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/depthType)"]
    pub fn depth_type(this: &XrSession) -> Option<XrDepthType>;
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "depthActive")]
    #[doc = "Getter for the `depthActive` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/depthActive)"]
    pub fn depth_active(this: &XrSession) -> Option<bool>;
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "domOverlayState")]
    #[doc = "Getter for the `domOverlayState` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/domOverlayState)"]
    pub fn dom_overlay_state(this: &XrSession) -> Option<XrdomOverlayState>;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRSession",
        js_name = "preferredReflectionFormat"
    )]
    #[doc = "Getter for the `preferredReflectionFormat` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/preferredReflectionFormat)"]
    pub fn preferred_reflection_format(this: &XrSession) -> XrReflectionFormat;
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "visibilityState")]
    #[doc = "Getter for the `visibilityState` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/visibilityState)"]
    pub fn visibility_state(this: &XrSession) -> XrVisibilityState;
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "frameRate")]
    #[doc = "Getter for the `frameRate` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/frameRate)"]
    pub fn frame_rate(this: &XrSession) -> Option<f32>;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRSession",
        js_name = "supportedFrameRates"
    )]
    #[doc = "Getter for the `supportedFrameRates` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/supportedFrameRates)"]
    pub fn supported_frame_rates(this: &XrSession) -> Option<::alloc::vec::Vec<f32>>;
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "renderState")]
    #[doc = "Getter for the `renderState` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/renderState)"]
    pub fn render_state(this: &XrSession) -> XrRenderState;
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "inputSources")]
    #[doc = "Getter for the `inputSources` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/inputSources)"]
    pub fn input_sources(this: &XrSession) -> XrInputSourceArray;
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "trackedSources")]
    #[doc = "Getter for the `trackedSources` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/trackedSources)"]
    pub fn tracked_sources(this: &XrSession) -> XrInputSourceArray;
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "enabledFeatures")]
    #[doc = "Getter for the `enabledFeatures` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/enabledFeatures)"]
    pub fn enabled_features(this: &XrSession) -> ::js_sys::Array;
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRSession",
        js_name = "isSystemKeyboardSupported"
    )]
    #[doc = "Getter for the `isSystemKeyboardSupported` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/isSystemKeyboardSupported)"]
    pub fn is_system_keyboard_supported(this: &XrSession) -> bool;
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "onend")]
    #[doc = "Getter for the `onend` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onend)"]
    pub fn onend(this: &XrSession) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, setter, js_class = "XRSession", js_name = "onend")]
    #[doc = "Setter for the `onend` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onend)"]
    pub fn set_onend(this: &XrSession, value: &::wasm_bindgen::JsValue);
    #[wasm_bindgen(
        method,
        getter,
        js_class = "XRSession",
        js_name = "oninputsourceschange"
    )]
    #[doc = "Getter for the `oninputsourceschange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/oninputsourceschange)"]
    pub fn oninputsourceschange(this: &XrSession) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(
        method,
        setter,
        js_class = "XRSession",
        js_name = "oninputsourceschange"
    )]
    #[doc = "Setter for the `oninputsourceschange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/oninputsourceschange)"]
    pub fn set_oninputsourceschange(this: &XrSession, value: &::wasm_bindgen::JsValue);
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "onselect")]
    #[doc = "Getter for the `onselect` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onselect)"]
    pub fn onselect(this: &XrSession) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, setter, js_class = "XRSession", js_name = "onselect")]
    #[doc = "Setter for the `onselect` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onselect)"]
    pub fn set_onselect(this: &XrSession, value: &::wasm_bindgen::JsValue);
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "onselectstart")]
    #[doc = "Getter for the `onselectstart` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onselectstart)"]
    pub fn onselectstart(this: &XrSession) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, setter, js_class = "XRSession", js_name = "onselectstart")]
    #[doc = "Setter for the `onselectstart` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onselectstart)"]
    pub fn set_onselectstart(this: &XrSession, value: &::wasm_bindgen::JsValue);
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "onselectend")]
    #[doc = "Getter for the `onselectend` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onselectend)"]
    pub fn onselectend(this: &XrSession) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, setter, js_class = "XRSession", js_name = "onselectend")]
    #[doc = "Setter for the `onselectend` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onselectend)"]
    pub fn set_onselectend(this: &XrSession, value: &::wasm_bindgen::JsValue);
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "onsqueeze")]
    #[doc = "Getter for the `onsqueeze` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onsqueeze)"]
    pub fn onsqueeze(this: &XrSession) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, setter, js_class = "XRSession", js_name = "onsqueeze")]
    #[doc = "Setter for the `onsqueeze` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onsqueeze)"]
    pub fn set_onsqueeze(this: &XrSession, value: &::wasm_bindgen::JsValue);
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "onsqueezestart")]
    #[doc = "Getter for the `onsqueezestart` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onsqueezestart)"]
    pub fn onsqueezestart(this: &XrSession) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, setter, js_class = "XRSession", js_name = "onsqueezestart")]
    #[doc = "Setter for the `onsqueezestart` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onsqueezestart)"]
    pub fn set_onsqueezestart(this: &XrSession, value: &::wasm_bindgen::JsValue);
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "onsqueezeend")]
    #[doc = "Getter for the `onsqueezeend` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onsqueezeend)"]
    pub fn onsqueezeend(this: &XrSession) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, setter, js_class = "XRSession", js_name = "onsqueezeend")]
    #[doc = "Setter for the `onsqueezeend` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onsqueezeend)"]
    pub fn set_onsqueezeend(this: &XrSession, value: &::wasm_bindgen::JsValue);
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "onvisibilitychange")]
    #[doc = "Getter for the `onvisibilitychange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onvisibilitychange)"]
    pub fn onvisibilitychange(this: &XrSession) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, setter, js_class = "XRSession", js_name = "onvisibilitychange")]
    #[doc = "Setter for the `onvisibilitychange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onvisibilitychange)"]
    pub fn set_onvisibilitychange(this: &XrSession, value: &::wasm_bindgen::JsValue);
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "onframeratechange")]
    #[doc = "Getter for the `onframeratechange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onframeratechange)"]
    pub fn onframeratechange(this: &XrSession) -> ::wasm_bindgen::JsValue;
    #[wasm_bindgen(method, setter, js_class = "XRSession", js_name = "onframeratechange")]
    #[doc = "Setter for the `onframeratechange` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/onframeratechange)"]
    pub fn set_onframeratechange(this: &XrSession, value: &::wasm_bindgen::JsValue);
    #[wasm_bindgen(method, getter, js_class = "XRSession", js_name = "maxRenderLayers")]
    #[doc = "Getter for the `maxRenderLayers` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/maxRenderLayers)"]
    pub fn max_render_layers(this: &XrSession) -> u32;
    #[wasm_bindgen(method, js_class = "XRSession", js_name = "cancelAnimationFrame")]
    #[doc = "The `cancelAnimationFrame()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/cancelAnimationFrame)"]
    pub fn cancel_animation_frame(this: &XrSession, handle: u32);
    #[wasm_bindgen(method, js_class = "XRSession")]
    #[doc = "The `end()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/end)"]
    pub fn end(this: &XrSession) -> ::js_sys::Promise;
    #[wasm_bindgen(method, js_class = "XRSession", js_name = "initiateRoomCapture")]
    #[doc = "The `initiateRoomCapture()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/initiateRoomCapture)"]
    pub fn initiate_room_capture(this: &XrSession) -> ::js_sys::Promise;
    #[wasm_bindgen(method, js_class = "XRSession", js_name = "pauseDepthSensing")]
    #[doc = "The `pauseDepthSensing()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/pauseDepthSensing)"]
    pub fn pause_depth_sensing(this: &XrSession);
    #[wasm_bindgen(method, js_class = "XRSession", js_name = "requestAnimationFrame")]
    #[doc = "The `requestAnimationFrame()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/requestAnimationFrame)"]
    pub fn request_animation_frame(this: &XrSession, callback: &::js_sys::Function) -> u32;
    #[wasm_bindgen(method, js_class = "XRSession", js_name = "requestHitTestSource")]
    #[doc = "The `requestHitTestSource()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/requestHitTestSource)"]
    pub fn request_hit_test_source(
        this: &XrSession,
        options: &XrHitTestOptionsInit,
    ) -> ::js_sys::Promise;
    #[wasm_bindgen(
        method,
        js_class = "XRSession",
        js_name = "requestHitTestSourceForTransientInput"
    )]
    #[doc = "The `requestHitTestSourceForTransientInput()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/requestHitTestSourceForTransientInput)"]
    pub fn request_hit_test_source_for_transient_input(
        this: &XrSession,
        options: &XrTransientInputHitTestOptionsInit,
    ) -> ::js_sys::Promise;
    #[wasm_bindgen(method, js_class = "XRSession", js_name = "requestLightProbe")]
    #[doc = "The `requestLightProbe()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/requestLightProbe)"]
    pub fn request_light_probe(this: &XrSession) -> ::js_sys::Promise;
    #[wasm_bindgen(method, js_class = "XRSession", js_name = "requestLightProbe")]
    #[doc = "The `requestLightProbe()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/requestLightProbe)"]
    pub fn request_light_probe_with_options(
        this: &XrSession,
        options: &XrLightProbeInit,
    ) -> ::js_sys::Promise;
    #[wasm_bindgen(method, js_class = "XRSession", js_name = "requestReferenceSpace")]
    #[doc = "The `requestReferenceSpace()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/requestReferenceSpace)"]
    pub fn request_reference_space(
        this: &XrSession,
        type_: XrReferenceSpaceType,
    ) -> ::js_sys::Promise;
    #[wasm_bindgen(method, js_class = "XRSession", js_name = "resumeDepthSensing")]
    #[doc = "The `resumeDepthSensing()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/resumeDepthSensing)"]
    pub fn resume_depth_sensing(this: &XrSession);
    #[wasm_bindgen(method, js_class = "XRSession", js_name = "updateRenderState")]
    #[doc = "The `updateRenderState()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/updateRenderState)"]
    pub fn update_render_state(this: &XrSession);
    #[wasm_bindgen(method, js_class = "XRSession", js_name = "updateRenderState")]
    #[doc = "The `updateRenderState()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/updateRenderState)"]
    pub fn update_render_state_with_state(this: &XrSession, state: &XrRenderStateInit);
    #[wasm_bindgen(method, js_class = "XRSession", js_name = "updateTargetFrameRate")]
    #[doc = "The `updateTargetFrameRate()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/XRSession/updateTargetFrameRate)"]
    pub fn update_target_frame_rate(this: &XrSession, rate: f32) -> ::js_sys::Promise;
}
