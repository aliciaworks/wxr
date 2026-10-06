//! What the room's light is, which is WebXR's lighting estimation.

use glam::Vec3;

/// A place to read the room's light from, which is WebXR's `XRLightProbe`.
///
/// A handle the way a hit-test source is, and for the same reason: a runtime makes one asynchronously, so what
/// a caller holds is its name.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct LightProbe {
    id: u32,
}

impl LightProbe {
    /// A probe a backend has handed out, named by whatever it uses to tell them apart.
    pub fn new(id: u32) -> Self {
        Self { id }
    }

    /// The backend's own name for it.
    pub fn id(self) -> u32 {
        self.id
    }
}

/// What the room's light is at one frame, which is WebXR's `XRLightEstimate`.
///
/// Nine spherical-harmonic coefficients and a primary light. The coefficients are the ambient light as a
/// function of direction - index 0 is the constant term and the rest are the low-frequency lobes - which is what
/// lets a virtual object be lit by the room instead of by a guess. Each is RGB, in WebXR's order: `C00`, `C1-1`,
/// `C10`, `C11`, `C2-2`, `C2-1`, `C20`, `C21`, `C22`.
///
/// Everything is expressed in the **probe's** space, which is the runtime's: WebXR makes a probe track a point
/// in the room and does not let the app choose it, and changing a spherical harmonic's basis is not a rotation
/// this core does. A scene that wants the primary light in its own frame brings it over itself - it is one
/// vector - while the ambient is low-frequency enough to use where it is.
///
/// All zero is a runtime with no estimate, which is a scene lit by nothing rather than by a guess.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct LightEstimate {
    /// A unit vector toward the brightest light, in the probe's space, and its colour per channel. WebXR's
    /// answer when it has no direction is straight down and no intensity, which is also what the default is.
    pub primary_direction: Vec3,
    pub primary_intensity: Vec3,
    /// The ambient light, as nine RGB coefficients in WebXR's order.
    pub harmonics: [Vec3; 9],
}
