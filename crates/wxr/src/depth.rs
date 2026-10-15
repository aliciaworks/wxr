//! How far away the real world is, which is WebXR's depth sensing.

use crate::space::Pose;
use crate::target::Extent2d;

/// What a runtime measured of the real world for one eye, which is WebXR's `XRDepthInformation`.
///
/// The shape of the depth buffer, what its values are, and the transform a shader needs to index it. The
/// buffer itself is not here but in [`crate::Session::Depth`] - this platform's own name for it, the same
/// split [`crate::Session::Image`] has for colour and for the same reason. What this is, is the part that
/// says what that buffer means.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct DepthInfo {
    /// How many columns and rows the depth buffer has.
    pub size: Extent2d,
    /// What the values in it are, which is what decides what has to be done to them.
    ///
    /// This is a choice rather than a scale because the two platforms do not agree and neither can answer in
    /// the other's terms. See [`DepthEncoding`] for the two, and for why a caller has to look at which one it
    /// got instead of multiplying by a number.
    pub encoding: DepthEncoding,
    /// From normalized view coordinates into the depth buffer's, so a point on the screen can be found in it.
    pub norm_depth_buffer_from_norm_view: Pose,
}

/// What the values in a depth buffer are, and therefore what turns one into metres.
///
/// A depth buffer is a texture of numbers and there is no standard answer to what the numbers mean - only the
/// two answers in here, which are what the two platforms this workspace speaks to actually give:
///
/// * **[`Linear`](DepthEncoding::Linear)**: the number is already a distance, up to one multiply. WebXR says
///   so with `XRDepthInformation.rawValueToMeters`, and it is the reason [`DepthInfo`] once had a plain
///   `raw_value_to_meters: f32` field and nothing else.
/// * **[`Window`](DepthEncoding::Window)**: the number is a *projection* of distance, which is what a depth
///   buffer is for - the rasterizer compares these values, and a comparison wants something whose order
///   matches the picture. `XR_META_environment_depth` gives this, with the two planes its frustum was placed
///   at and no scalar, because a hyperbolic function has no scalar.
///
/// [`Unknown`](DepthEncoding::Unknown) is neither, and it is not a placeholder: it is a buffer whose meaning
/// was never stated, which a caller has to treat as saying nothing. The renderer's occlusion shader checks for
/// it first and draws the scene, which is the picture a session with no depth at all produces - a wrong
/// distance would be a picture of something else entirely, and nobody could tell which.
///
/// The reason this is an enum and not two optional fields is that the two encodings cannot both be right and a
/// caller that has to remember which one to consult first is a caller that will one day get it the other way
/// round. `match` on this and the compiler names the case that was forgotten.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum DepthEncoding {
    /// `metres = raw_value * to_meters`, which is WebXR's `rawValueToMeters`.
    ///
    /// The scalar can be anything, including less than one: the specification does not constrain it, and a
    /// buffer scaled down to a fixed point is the whole reason it exists rather than the values being metres.
    Linear { to_meters: f32 },

    /// A window-space depth over the frustum's two planes, which is what OpenXR's environment depth is.
    ///
    /// The buffer holds `near_z` and `far_z` as *distances in metres*, and the value at a texel is where
    /// between those two planes the wall was. Going back is the inverse of the projection that made it, which
    /// `XR_KHR_composition_layer_depth` states as a homogeneous transform over four numbers -
    /// `minDepth`/`maxDepth`/`nearZ`/`farZ` - of which this extension reports only the last two, with the
    /// first two being the near and far frustum planes *respectively* and therefore 0 and 1. So:
    ///
    /// ```text
    /// metres = near_z * far_z / (far_z - value * (far_z - near_z))
    /// ```
    ///
    /// which is `near_z` at a value of 0 and `far_z` at 1, as a depth buffer that puts near at zero does.
    ///
    /// **`far_z < near_z` is not a reversed depth buffer, it is an infinite one**: `XR_META_environment_depth`
    /// says so in as many words - "When farZ is less than nearZ, an infinite projection matrix is used" - and
    /// the finite formula above has no opinion about which end of the range is nearer, so a caller has to
    /// check for it rather than trust it. What it becomes is `near_z / (1 - value)`, the same formula with
    /// `far_z` at infinity.
    Window { near_z: f32, far_z: f32 },

    /// Nothing said what the values are, so nothing may be assumed about them.
    ///
    /// The default, and the honest one: a session that has not measured anything yet answers with this rather
    /// than with a number a caller would believe.
    #[default]
    Unknown,
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
