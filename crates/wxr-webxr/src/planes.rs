//! The surfaces WebXR detects, read off the frame.
//!
//! Plane detection is a module of its own and `web-sys` generates nothing for it, so every field here is read by
//! name - the way this backend already reads `environmentBlendMode` and `targetRayMode`. The one thing that is
//! not a name is a pose, so a plane's space goes to `getPose` like any other space.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::{XrFrame, XrReferenceSpace, XrSpace};

/// The identity of the surfaces this session has seen.
///
/// WebXR names a plane by the object it is, the way it does an input source and for the same reason - so this is
/// the map that turns one into the number a core [`wxr::Plane`] carries.
#[derive(Clone, Default)]
pub struct Ids {
    known: Rc<RefCell<Vec<JsValue>>>,
}

impl Ids {
    /// The core's id for a plane, remembering it if it has not been seen before.
    fn id(&self, plane: &JsValue) -> u32 {
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
    // `detectedPlanes` is null for a session that was not granted the feature, which is a frame with no surfaces
    // rather than a frame to fail.
    let Ok(planes) = js_sys::Reflect::get(frame, &JsValue::from_str("detectedPlanes")) else {
        return;
    };
    let Ok(Some(planes)) = js_sys::try_iter(&planes) else {
        return;
    };
    for plane in planes.flatten() {
        let Ok(space) = js_sys::Reflect::get(&plane, &JsValue::from_str("planeSpace")) else {
            continue;
        };
        // A plane with no pose this frame is a plane the runtime is no longer sure of, and one that is not
        // reported is better than one reported at the origin.
        let Some(pose) =
            frame.get_pose(space.unchecked_ref::<XrSpace>(), reference.unchecked_ref())
        else {
            continue;
        };
        into.push(wxr::Plane {
            id: ids.id(&plane),
            pose: crate::transform(pose.transform()),
            extent: extent(&plane),
            orientation: orientation(&plane),
        });
    }
}

/// Which way up the surface is, which WebXR gives as one of two strings.
fn orientation(plane: &JsValue) -> wxr::PlaneOrientation {
    match js_sys::Reflect::get(plane, &JsValue::from_str("orientation"))
        .ok()
        .and_then(|value| value.as_string())
        .as_deref()
    {
        Some("horizontal") => wxr::PlaneOrientation::Horizontal,
        Some("vertical") => wxr::PlaneOrientation::Vertical,
        _ => wxr::PlaneOrientation::Unknown,
    }
}

/// The bounding box of the plane's outline, which WebXR reports instead of a size.
///
/// The points are in the plane's own space, so the box is across X and along Z and the width and height fall
/// out of the extremes. An outline that is empty or not there is a zero extent, which is a surface a caller can
/// still avoid - the extent is a size, not a promise.
fn extent(plane: &JsValue) -> wxr::glam::Vec2 {
    let Ok(polygon) = js_sys::Reflect::get(plane, &JsValue::from_str("polygon")) else {
        return wxr::glam::Vec2::ZERO;
    };
    let Ok(Some(points)) = js_sys::try_iter(&polygon) else {
        return wxr::glam::Vec2::ZERO;
    };
    let mut min = wxr::glam::Vec2::splat(f32::MAX);
    let mut max = wxr::glam::Vec2::splat(f32::MIN);
    for point in points.flatten() {
        let at = |name: &str| {
            js_sys::Reflect::get(&point, &JsValue::from_str(name))
                .ok()
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0) as f32
        };
        let (x, z) = (at("x"), at("z"));
        min = min.min(wxr::glam::Vec2::new(x, z));
        max = max.max(wxr::glam::Vec2::new(x, z));
    }
    if min.x > max.x || min.y > max.y {
        return wxr::glam::Vec2::ZERO;
    }
    max - min
}
