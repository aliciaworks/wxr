//! The surfaces WebXR detects, read off the frame.
//!
//! Plane detection is a module whose IDL is part of the generated snapshot, so nothing here is read by name.
//! What is left is the identity of the surfaces: WebXR names a plane by the object it is, the way it does an
//! input source, and a core [`wxr::Plane`] carries a number.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;

use crate::sys::{DomPointReadOnly, XrFrame, XrPlane, XrPlaneOrientation, XrReferenceSpace};

/// The identity of the surfaces this session has seen.
#[derive(Clone, Default)]
pub struct Ids {
    known: Rc<RefCell<Vec<XrPlane>>>,
}

impl Ids {
    /// The core's id for a plane, remembering it if it has not been seen before.
    fn id(&self, plane: &XrPlane) -> u32 {
        let mut known = self.known.borrow_mut();
        if let Some(index) = known.iter().position(|seen| seen == plane) {
            return index as u32;
        }
        known.push(plane.clone());
        (known.len() - 1) as u32
    }
}

/// Read the frame's detected surfaces into the core's shape, in the space given.
pub fn detected(
    frame: &XrFrame,
    reference: &XrReferenceSpace,
    ids: &Ids,
    into: &mut Vec<wxr::Plane>,
) {
    // The set is empty for a session that was not granted the feature, which is a frame with no surfaces rather
    // than a frame to fail. Its `values()` is what iterates it, which `try_iter` is how to reach from Rust.
    let planes = frame.detected_planes();
    let Ok(Some(planes)) = js_sys::try_iter(planes.as_ref()) else {
        return;
    };
    for plane in planes.flatten() {
        let Ok(plane) = plane.dyn_into::<XrPlane>() else {
            continue;
        };
        // A plane the runtime will not place this frame is one it is no longer sure of, and one that is not
        // reported is better than one reported at the origin.
        let Some(pose) = frame.get_pose(&plane.plane_space(), reference) else {
            continue;
        };
        into.push(wxr::Plane {
            id: ids.id(&plane),
            pose: crate::transform(pose.transform()),
            extent: extent(&plane),
            orientation: match plane.orientation() {
                Some(XrPlaneOrientation::Horizontal) => wxr::PlaneOrientation::Horizontal,
                Some(XrPlaneOrientation::Vertical) => wxr::PlaneOrientation::Vertical,
                Some(XrPlaneOrientation::__Invalid) | None => wxr::PlaneOrientation::Unknown,
            },
        });
    }
}

/// The bounding box of the plane's outline, which WebXR reports instead of a size.
///
/// The points are in the plane's own space, so the box is across X and along Z and the width and height fall
/// out of the extremes. An outline that is empty or not there is a zero extent, which is a surface a caller can
/// still avoid - the extent is a size, not a promise.
fn extent(plane: &XrPlane) -> wxr::glam::Vec2 {
    let mut min = wxr::glam::Vec2::splat(f32::MAX);
    let mut max = wxr::glam::Vec2::splat(f32::MIN);
    for point in plane.polygon().iter() {
        let Ok(point) = point.dyn_into::<DomPointReadOnly>() else {
            continue;
        };
        let (x, z) = (point.x() as f32, point.z() as f32);
        min = min.min(wxr::glam::Vec2::new(x, z));
        max = max.max(wxr::glam::Vec2::new(x, z));
    }
    if min.x > max.x || min.y > max.y {
        return wxr::glam::Vec2::ZERO;
    }
    max - min
}
