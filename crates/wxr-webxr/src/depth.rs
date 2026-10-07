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
    Some((
        information.texture(),
        wxr::DepthInfo {
            size: wxr::Extent2d::new(information.width(), information.height()),
            raw_value_to_meters: information.raw_value_to_meters(),
            // The transform is an `XRRigidTransform`, the same shape a view's pose comes in.
            norm_depth_buffer_from_norm_view: crate::transform(
                information.norm_depth_buffer_from_norm_view(),
            ),
        },
    ))
}
