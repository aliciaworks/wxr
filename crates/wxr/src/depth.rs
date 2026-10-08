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

/// How a session's depth is optimized, which is WebXR's `XRDepthUsage`.
///
/// `CpuOptimized` is depth that arrives as something to read - the values in a buffer - and `GpuOptimized` is
/// depth the GPU samples where it lies. It is about where the values *are* and not about what they mean, and a
/// runtime reports it because it is the one that knows which of the two it can hand over.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DepthUsage {
    CpuOptimized,
    GpuOptimized,
}

/// Whether a session's depth was smoothed, which is WebXR's `XRDepthType`.
///
/// `Raw` is what a sensor measured and `Smooth` is a surface fitted to it: the first is honest about noise, the
/// second is what a picture that does not want to fight noise draws with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DepthType {
    Raw,
    Smooth,
}

/// What a session's depth values are, which is WebXR's `XRDepthDataFormat`.
///
/// Three, and each is a fact a shader has to be told: `LuminanceAlpha` packs one value into two channels,
/// `Float32` is the value, and `UnsignedShort` is it scaled into a fixed point.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DepthFormat {
    LuminanceAlpha,
    Float32,
    UnsignedShort,
}

/// What a session's depth sensing is: WebXR's four attributes of it, as one value.
///
/// Kept together because they are one answer to one question - what depth this session has - and a caller that
/// reads one of them reads all of them. The two `Option`s are the specification's own: a runtime may not say
/// which kind of depth it is, and may not say whether it is being delivered yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DepthSensing {
    pub usage: DepthUsage,
    pub format: DepthFormat,
    /// Which kind it is, where the runtime says - WebXR's `depthType`.
    pub ty: Option<DepthType>,
    /// Whether depth is being delivered - WebXR's `depthActive`.
    pub active: Option<bool>,
}
