//! How far away the real world is, which is WebXR's depth sensing.

use crate::space::Pose;
use crate::target::Extent2d;

/// What a runtime measured of the real world for one eye, which is WebXR's `XRDepthInformation`.
///
/// The shape of the depth buffer, what a raw value in it is worth, and the transform a shader needs to index
/// it. The buffer itself is deliberately not here: a CPU one is an `ArrayBuffer` whose layout is the session's
/// data format, and a GPU one is a texture the runtime destroys at the end of the frame it was made in - both
/// are a renderer's business, and what the core carries is the part that says what they mean.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct DepthInfo {
    /// How many columns and rows the depth buffer has.
    pub size: Extent2d,
    /// What one raw value in it is worth, in metres.
    pub raw_value_to_meters: f32,
    /// From normalized view coordinates into the depth buffer's, so a point on the screen can be found in it.
    pub norm_depth_buffer_from_norm_view: Pose,
}
