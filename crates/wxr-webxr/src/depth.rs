//! The real world's depth, which is WebXR's depth sensing.
//!
//! The GPU side of it, because that is the side that hands over *depth* rather than bytes: a `GPUTexture` the
//! runtime makes for the frame and takes back at the end of it, which is what a renderer can actually test
//! against. The CPU side - an `ArrayBuffer` and `getDepthInMeters` - is the web platform's way of delivering
//! the same thing when it cannot give a texture, and it is deliberately not in the core: what a caller wants is
//! the depth, not the delivery. A backend that had nothing but bytes would read them into a buffer itself, the
//! way wgpu hides a staging copy behind `map_async`.
//!
//! `XRGPUDepthInformation` is not in `web-sys` any more than the rest of the binding is, so it is read by name -
//! and it is asked of the *binding* rather than the frame, which is where the specification puts the WebGPU form.

use wasm_bindgen::prelude::*;
use web_sys::XrView;

use crate::gpu::XRGPUBinding;

/// The depth buffer for `view`, and what it means, from the binding that hands it over.
pub fn information(binding: &XRGPUBinding, view: &XrView) -> Option<(JsValue, wxr::DepthInfo)> {
    // It throws when the session was not configured for GPU depth, which is why it is caught: a session with
    // depth it cannot read is a session without depth, not a frame to fail.
    let information = binding.get_depth_information(view).ok()?;
    if information.is_null_or_undefined() {
        return None;
    }

    let field = |name: &str| {
        js_sys::Reflect::get(&information, &JsValue::from_str(name))
            .ok()
            .and_then(|value| value.as_f64())
    };
    // The transform is an `XRRigidTransform`, the same shape a view's pose comes in.
    let transform = js_sys::Reflect::get(
        &information,
        &JsValue::from_str("normDepthBufferFromNormView"),
    )
    .ok()?
    .dyn_into::<web_sys::XrRigidTransform>()
    .ok()?;
    let texture = js_sys::Reflect::get(&information, &JsValue::from_str("texture")).ok()?;

    Some((
        texture,
        wxr::DepthInfo {
            size: wxr::Extent2d::new(
                field("width").unwrap_or(0.0).max(0.0) as u32,
                field("height").unwrap_or(0.0).max(0.0) as u32,
            ),
            raw_value_to_meters: field("rawValueToMeters").unwrap_or(1.0) as f32,
            norm_depth_buffer_from_norm_view: crate::transform(transform),
        },
    ))
}
