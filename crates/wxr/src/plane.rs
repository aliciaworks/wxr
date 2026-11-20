//! The room the session is in, as far as the runtime can tell.

use std::time::Duration;

use glam::Vec2;

use crate::mesh::SurfaceKind;
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
/// A bounding box every backend can fill, and the outline when the runtime reports one: WebXR's polygon and
/// OpenXR's contour are the shape, and ARKit's plane anchors are read as a rectangle. The pose's orientation
/// puts the surface's normal on **+Y**, which is what WebXR's plane space says and what the other two are
/// brought to, and both the extent and the outline are in the plane's own X and Z, in metres.
#[derive(Clone, PartialEq, Debug)]
pub struct Plane {
    /// The backend's own identity for it, stable while the surface is tracked - WebXR's `XRPlane` object,
    /// OpenXR's entity, ARKit's anchor. The same kind of name [`crate::InputId`] is for a source, and for the
    /// same reason: a plane compared by value is two poses compared.
    pub id: u32,
    /// Where the surface's centre is, in whichever reference space it was asked for.
    pub pose: Pose,
    /// How big it is, in metres: `x` across and `y` along, in the plane's own axes.
    ///
    /// The bounding box of [`Plane::outline`] when there is one, so a caller that only needs a size - a
    /// quick overlap test before drawing, a footprint - does not have to look at the points.
    pub extent: Vec2,
    /// The boundary of the surface, when the runtime reports the shape rather than just a box: the points in
    /// order around it, in the plane's own X and Z, in metres.
    ///
    /// The loop is closed: the last point joins the first, and the first is not repeated. `None` is a runtime
    /// that reports only an extent - ARKit's plane anchors are read as a rectangle - and a caller that draws a
    /// surface with no outline draws [`Plane::extent`] instead. The two never disagree: an outline that is
    /// present is the shape the extent bounds.
    pub outline: Option<Vec<Vec2>>,
    pub orientation: PlaneOrientation,
    /// What the runtime says it is, when it says - WebXR's `semanticLabel`, OpenXR's `semantic_type`,
    /// ARKit's plane classification.
    ///
    /// The same list a mesh's patches are labelled with, because a floor is a floor whether the runtime
    /// boxed it or traced it. [`SurfaceKind::None`] is a runtime that found the surface and did not
    /// classify it, which is the answer ARKit gives before it has looked long enough to know.
    pub kind: SurfaceKind,
    /// When it last changed, on the same clock as the frames - WebXR's `XRPlane.lastChangedTime`, ARKit's
    /// anchor timestamp.
    ///
    /// `None` is a runtime that does not keep one, which is OpenXR's plane detector: it reports a pose and
    /// an extent and no clock. Saying zero there would be a time that never happened. A caller that rebuilds
    /// what sits on a surface on this is a caller that is not rebuilding it every frame.
    pub last_changed: Option<Duration>,
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
            outline: None,
            orientation: PlaneOrientation::default(),
            kind: SurfaceKind::default(),
            last_changed: None,
        };
        assert_eq!(plane.orientation, PlaneOrientation::Unknown);
        assert_eq!(plane.kind, SurfaceKind::None);
        assert_eq!(plane.last_changed, None);
        assert_eq!(plane.outline, None);
        assert!(plane.extent.x > plane.extent.y);
    }

    /// A plane with an outline is still a plane without one when the shape is not reported, and the extent
    /// stays the size a caller can test against.
    #[test]
    fn an_outline_is_the_shape_the_extent_bounds() {
        let outline = vec![
            Vec2::new(-0.5, -0.25),
            Vec2::new(0.5, -0.25),
            Vec2::new(0.5, 0.25),
            Vec2::new(-0.5, 0.25),
        ];
        let plane = Plane {
            id: 1,
            pose: Pose::IDENTITY,
            extent: Vec2::new(1.0, 0.5),
            outline: Some(outline.clone()),
            orientation: PlaneOrientation::Horizontal,
            kind: SurfaceKind::Floor,
            last_changed: None,
        };
        let points = plane.outline.as_deref().unwrap_or_default();
        let min = points
            .iter()
            .copied()
            .fold(Vec2::splat(f32::MAX), Vec2::min);
        let max = points
            .iter()
            .copied()
            .fold(Vec2::splat(f32::MIN), Vec2::max);
        assert_eq!(max - min, plane.extent);
        // Cloned rather than copied: a plane's outline is a list, and two planes that share one are still
        // two answers about a frame.
        assert_eq!(plane.clone().outline, Some(outline));
    }
}
