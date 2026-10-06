//! How far away the real world is, which is WebXR's depth sensing.

use crate::space::Pose;
use crate::target::Extent2d;

/// What a runtime measured of the real world for one eye, which is WebXR's `XRDepthInformation`.
///
/// The shape of the depth buffer, what a raw value in it is worth, and the transform a shader needs to index
/// it. The buffer itself is not here but in [`crate::Session::Depth`] - this platform's own name for it, the
/// same split [`crate::Session::Image`] has for colour and for the same reason. What this is, is the part that
/// says what that buffer means.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct DepthInfo {
    /// How many columns and rows the depth buffer has.
    pub size: Extent2d,
    /// What one raw value in it is worth, in metres.
    pub raw_value_to_meters: f32,
    /// From normalized view coordinates into the depth buffer's, so a point on the screen can be found in it.
    pub norm_depth_buffer_from_norm_view: Pose,
}
