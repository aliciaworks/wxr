//! The room the session is in, as far as the runtime can tell.

use glam::Vec2;

use crate::space::Pose;

/// How a surface lies, which is WebXR's `XRPlaneOrientation`.
///
/// `Unknown` is a runtime that found the surface and will not say which way up it is - a real answer, and not a
/// failure: a plane to avoid walking through is worth having before it is worth drawing on.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PlaneOrientation {
    Horizontal,
    Vertical,
    #[default]
    Unknown,
}

/// A surface the runtime has found, which is WebXR's `XRPlane`.
///
/// A bounding box and not the outline: WebXR reports a polygon, OpenXR an extent, ARKit an extent - so the box
/// is the thing all three have, and the outline is the platform's. The pose's orientation puts the surface's
/// normal on **+Y**, which is what WebXR's plane space says and what the other two are brought to, and the
/// extent is measured in the plane's own X and Z, in metres.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Plane {
    /// The backend's own identity for it, stable while the surface is tracked - WebXR's `XRPlane` object,
    /// OpenXR's entity, ARKit's anchor. The same kind of name [`crate::InputId`] is for a source, and for the
    /// same reason: a plane compared by value is two poses compared.
    pub id: u32,
    /// Where the surface's centre is, in whichever reference space it was asked for.
    pub pose: Pose,
    /// How big it is, in metres: `x` across and `y` along, in the plane's own axes.
    pub extent: Vec2,
    pub orientation: PlaneOrientation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plane_without_an_orientation_is_still_a_plane() {
        let plane = Plane {
            id: 0,
            pose: Pose::IDENTITY,
            extent: Vec2::new(1.5, 0.8),
            orientation: PlaneOrientation::default(),
        };
        assert_eq!(plane.orientation, PlaneOrientation::Unknown);
        assert!(plane.extent.x > plane.extent.y);
    }
}
