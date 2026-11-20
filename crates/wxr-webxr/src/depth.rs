//! The real world's depth, which is WebXR's depth sensing.
//!
//! The GPU side of it, because that is the side that hands over *depth* rather than bytes: a texture the
//! runtime makes for the frame and takes back at the end of it, which is what a renderer can actually test
//! against. The CPU side - an `ArrayBuffer` and `getDepthInMeters` - is the web platform's way of delivering
//! the same thing when it cannot give a texture, and it is deliberately not in the core: what a caller wants is
//! the depth, not the delivery. A backend that had nothing but bytes would read them into a buffer itself, the
//! way wgpu hides a staging copy behind `map_async`.
//!
//! `XRGPUDepthInformation` is asked of the *binding* rather than the frame, which is where the specification
//! puts the WebGPU form of it, and it is asked through a caught call: a session not configured for GPU depth
//! throws from it, which is a session without depth rather than a frame to fail.

use wasm_bindgen::prelude::*;

use crate::sys::{XrView, XrgpuBinding};
use crate::throws;

/// The depth buffer for `view`, and what it means, from the binding that hands it over.
pub fn information(binding: &XrgpuBinding, view: &XrView) -> Option<(JsValue, wxr::DepthInfo)> {
    let information = throws::get_depth_information(binding, view).ok()?;
    let width = information.width();
    let height = information.height();
    // A buffer with no pixels is not a depth buffer, and wrapping one is a texture the renderer cannot make.
    // The browser's own value is the only size there is, so the honest answer is that this view has none.
    if width == 0 || height == 0 {
        return None;
    }
    let texture = information.texture();
    // The format is the texture's, and it is the one thing that decides whether a shader can read this at all.
    // `float32` is `r32float`, a single floating-point channel the occlusion pass samples as one number;
    // `unsigned-short` is `r16uint`, which is an integer channel with a `textureLoad` and a cast in front of
    // it, and it is what a browser that would not grant `float32` hands over. Reading that as a float would be
    // a distance from some other number entirely, so the answer is no depth - the same answer as a session
    // that measured nothing, and a frame a scene draws everything into.
    if depth_format(&texture).as_deref() != Some("r32float") {
        return None;
    }
    Some((
        texture,
        wxr::DepthInfo {
            size: wxr::Extent2d::new(width, height),
            // One scalar and one multiply, which is what the specification says this is: the buffer's values
            // are distances, up to `rawValueToMeters`. WebXR has no equivalent of OpenXR's two planes, and
            // this backend would have nothing to report for a `Window` encoding.
            encoding: wxr::DepthEncoding::Linear {
                to_meters: information.raw_value_to_meters(),
            },
            // The transform is an `XRRigidTransform`, the same shape a view's pose comes in.
            norm_depth_buffer_from_norm_view: crate::transform(
                information.norm_depth_buffer_from_norm_view(),
            ),
        },
    ))
}

/// The `GPUTextureFormat` of a browser depth texture, as the browser spells it.
///
/// A `GPUTexture` is deliberately not one of the generated types - it belongs to wgpu and to the page - so its
/// fields are read by name, which is the same read `session::image_meta` makes of a colour texture.
fn depth_format(texture: &JsValue) -> Option<String> {
    js_sys::Reflect::get(texture, &JsValue::from_str("format"))
        .ok()?
        .as_string()
}
