//! The calls whose throw the specification only states in prose.
//!
//! `wasm-bindgen` catches a call when its IDL says `[Throws]`, and the WebXR specifications mostly do not say
//! it there: they describe the exception in the algorithm and leave the signature ordinary. A generated method
//! for one of those is a method that, when the browser throws, throws *through* this crate and out of whatever
//! called it - which in a frame loop is not a frame that failed but a frame that stopped happening.
//!
//! So this is the list of them: every call this backend makes whose throw the specification states. It is
//! deliberately short, and it adds no names - the generated module has all of them, from the IDL, and a
//! misspelling is still impossible. What is added is the `catch`, which is the one thing generation cannot
//! see, and each one is written down with the exception it is for.

use wasm_bindgen::prelude::*;

use crate::sys::{
    XrAnchor, XrCompositionLayer, XrEye, XrFrame, XrProjectionLayer, XrRenderStateInit, XrSession,
    XrSpace, XrView, XrgpuBinding, XrgpuDepthInformation, XrgpuProjectionLayerInit,
    XrgpuQuadLayerInit, XrgpuSubImage,
};

#[wasm_bindgen]
extern "C" {
    /// `XRAnchor.anchorSpace`, which throws `InvalidStateError` once the anchor is deleted - a place that is
    /// gone rather than a frame to fail.
    #[wasm_bindgen(method, getter, catch, js_name = "anchorSpace")]
    fn anchor_space_caught(this: &XrAnchor) -> Result<XrSpace, JsValue>;

    /// `XRSession.updateRenderState`, which throws `InvalidStateError` during an animation frame. A layer list
    /// is handed over between frames, and a caller that breaks that rule gets `Err` rather than a trap.
    #[wasm_bindgen(method, catch, js_name = "updateRenderState")]
    fn update_render_state_caught(
        this: &XrSession,
        state: &XrRenderStateInit,
    ) -> Result<(), JsValue>;

    /// `XRGPUBinding.createProjectionLayer`: `NotSupportedError` for a format the device will not take, and
    /// `InvalidStateError` for a session that has ended.
    #[wasm_bindgen(method, catch, js_name = "createProjectionLayer")]
    fn create_projection_layer_caught(
        this: &XrgpuBinding,
        init: &XrgpuProjectionLayerInit,
    ) -> Result<XrProjectionLayer, JsValue>;

    /// `XRGPUBinding.createQuadLayer`, throwing for the same reasons.
    #[wasm_bindgen(method, catch, js_name = "createQuadLayer")]
    fn create_quad_layer_caught(
        this: &XrgpuBinding,
        init: &XrgpuQuadLayerInit,
    ) -> Result<crate::sys::XrQuadLayer, JsValue>;

    /// `XRGPUBinding.getViewSubImage`, which throws when the layer is not the projection layer of this session.
    #[wasm_bindgen(method, catch, js_name = "getViewSubImage")]
    fn get_view_sub_image_caught(
        this: &XrgpuBinding,
        layer: &XrProjectionLayer,
        view: &XrView,
    ) -> Result<XrgpuSubImage, JsValue>;

    /// `XRGPUBinding.getSubImage`, which throws when the layer is not one this session presents.
    #[wasm_bindgen(method, catch, js_name = "getSubImage")]
    fn get_sub_image_caught(
        this: &XrgpuBinding,
        layer: &XrCompositionLayer,
        frame: &XrFrame,
        eye: XrEye,
    ) -> Result<XrgpuSubImage, JsValue>;

    /// `XRGPUBinding.getDepthInformation`, which throws for a session that was not configured for GPU depth -
    /// which is the answer `Session::depth` gives as `None`.
    #[wasm_bindgen(method, catch, js_name = "getDepthInformation")]
    fn get_depth_information_caught(
        this: &XrgpuBinding,
        view: &XrView,
    ) -> Result<XrgpuDepthInformation, JsValue>;
}

// The wrappers, under the names the generated module would have given them: a caller should not have to know
// that a throw had to be caught, only that the call can fail.
//
// They exist because `wasm-bindgen` will not let one type have two methods with the same Rust name, so the
// caught binding above is spelled differently and this is where the difference stops.

/// `XRAnchor.anchorSpace`, caught.
pub fn anchor_space(anchor: &XrAnchor) -> Result<XrSpace, JsValue> {
    anchor.anchor_space_caught()
}

/// `XRSession.updateRenderState`, caught.
pub fn update_render_state(session: &XrSession, state: &XrRenderStateInit) -> Result<(), JsValue> {
    session.update_render_state_caught(state)
}

/// `XRGPUBinding.createProjectionLayer`, caught.
pub fn create_projection_layer(
    binding: &XrgpuBinding,
    init: &XrgpuProjectionLayerInit,
) -> Result<XrProjectionLayer, JsValue> {
    binding.create_projection_layer_caught(init)
}

/// `XRGPUBinding.createQuadLayer`, caught.
pub fn create_quad_layer(
    binding: &XrgpuBinding,
    init: &XrgpuQuadLayerInit,
) -> Result<crate::sys::XrQuadLayer, JsValue> {
    binding.create_quad_layer_caught(init)
}

/// `XRGPUBinding.getViewSubImage`, caught.
pub fn get_view_sub_image(
    binding: &XrgpuBinding,
    layer: &XrProjectionLayer,
    view: &XrView,
) -> Result<XrgpuSubImage, JsValue> {
    binding.get_view_sub_image_caught(layer, view)
}

/// `XRGPUBinding.getSubImage`, caught.
pub fn get_sub_image(
    binding: &XrgpuBinding,
    layer: &XrCompositionLayer,
    frame: &XrFrame,
    eye: XrEye,
) -> Result<XrgpuSubImage, JsValue> {
    binding.get_sub_image_caught(layer, frame, eye)
}

/// `XRGPUBinding.getDepthInformation`, caught.
pub fn get_depth_information(
    binding: &XrgpuBinding,
    view: &XrView,
) -> Result<XrgpuDepthInformation, JsValue> {
    binding.get_depth_information_caught(view)
}
