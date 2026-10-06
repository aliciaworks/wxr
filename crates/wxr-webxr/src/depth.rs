//! The real world's depth, which is WebXR's depth sensing.
//!
//! `depth-sensing` is another module `web-sys` generates nothing for, so a depth buffer is read by name like
//! the rest. Only the CPU side is here: the buffer is an `ArrayBuffer` the browser fills per view. The GPU side
//! is a texture the runtime destroys at the end of the frame it was made in, which is a renderer's to import
//! and not a core's to carry.

use wasm_bindgen::prelude::*;
use web_sys::{XrFrame, XrView};

/// A method by name, which is the only way to reach a module `web-sys` does not know.
fn method(target: &JsValue, name: &str) -> Option<js_sys::Function> {
    js_sys::Reflect::get(target, &JsValue::from_str(name))
        .ok()?
        .dyn_into()
        .ok()
}

/// The depth information this frame has for `view`, if it has any.
///
/// `null` is a frame with none - a runtime that paused depth sensing, or one with no sensor pointing there -
/// which is not an error to fail a frame over.
fn information(frame: &XrFrame, view: &XrView) -> Option<JsValue> {
    let call = method(frame.unchecked_ref::<JsValue>(), "getDepthInformation")?;
    let value = call
        .call1(
            frame.unchecked_ref::<JsValue>(),
            view.unchecked_ref::<JsValue>(),
        )
        .ok()?;
    (!value.is_null_or_undefined()).then_some(value)
}

/// What the description of the buffer says, or `None` if there is none this frame.
pub fn info(frame: &XrFrame, view: &XrView) -> Option<wxr::DepthInfo> {
    let information = information(frame, view)?;
    let field = |name: &str| {
        js_sys::Reflect::get(&information, &JsValue::from_str(name))
            .ok()
            .and_then(|value| value.as_f64())
    };
    // The transform is an `XRRigidTransform`, which is the same shape a view's pose comes in.
    let transform = js_sys::Reflect::get(
        &information,
        &JsValue::from_str("normDepthBufferFromNormView"),
    )
    .ok()?
    .dyn_into::<web_sys::XrRigidTransform>()
    .ok()?;
    Some(wxr::DepthInfo {
        size: wxr::Extent2d::new(
            field("width").unwrap_or(0.0).max(0.0) as u32,
            field("height").unwrap_or(0.0).max(0.0) as u32,
        ),
        raw_value_to_meters: field("rawValueToMeters").unwrap_or(1.0) as f32,
        norm_depth_buffer_from_norm_view: crate::transform(transform),
    })
}

/// How far away the world is along a ray through `view` at normalized view coordinates, in metres.
///
/// The one question the CPU side answers without the caller unpacking a buffer: `getDepthInMeters`, which is
/// where the scaling by `raw_value_to_meters` and the transform into the buffer's coordinates both happen.
pub fn at(frame: &XrFrame, view: &XrView, x: f32, y: f32) -> Option<f32> {
    let information = information(frame, view)?;
    let call = method(&information, "getDepthInMeters")?;
    let value = call
        .call2(
            &information,
            &JsValue::from_f64(x as f64),
            &JsValue::from_f64(y as f64),
        )
        .ok()?;
    value.as_f64().map(|metres| metres as f32)
}
