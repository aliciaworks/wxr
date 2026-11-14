//! Hit testing: where a ray out of a space meets the world.
//!
//! `XRHitTestSource` is created asynchronously and is otherwise opaque, so one waits in a slot until the
//! browser answers - and a frame with no source yet is a frame with no hits rather than a frame to fail.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::sys::{
    DomPointInit, XrFrame, XrHitTestOptionsInit, XrHitTestResult, XrHitTestSource, XrRay,
    XrRayDirectionInit, XrReferenceSpace, XrSession,
};

/// A source that has been asked for, and the browser's answer once it arrives.
#[derive(Default)]
pub struct Slot {
    source: Option<XrHitTestSource>,
    /// Set by [`Slot::cancel`], so an answer arriving afterwards fills nothing. A source let go while its
    /// request was still a promise would otherwise come back from having been released, which is the one
    /// way this slot can be wrong - the handle is still in the caller's hand either way.
    cancelled: bool,
}

impl Slot {
    /// The source, if the browser has answered and nobody has let it go.
    pub fn source(&self) -> Option<XrHitTestSource> {
        self.source.clone()
    }

    /// Tell the browser this source is done - `XRHitTestSource.cancel` - and forget it.
    ///
    /// Both halves are needed. An answered source would otherwise be one the browser keeps reporting for
    /// the rest of the session, and an unanswered one must not be filled in when its promise lands - so the
    /// flag outlives the promise, which is what makes a release at any moment the same release.
    pub fn cancel(&mut self) {
        self.cancelled = true;
        if let Some(source) = self.source.take() {
            source.cancel();
        }
    }

    /// Take the browser's answer, unless the source was let go while its request was in flight - in which
    /// case it is cancelled here rather than kept, since no caller will ever ask for it.
    pub fn fill(&mut self, source: XrHitTestSource) {
        if self.cancelled {
            source.cancel();
        } else {
            self.source = Some(source);
        }
    }
}

/// Ask the browser for a source aiming out of `space`, along `ray` when the app gave one, and fill the slot
/// when it answers.
///
/// Everything else about the options has a default, so there are two: the space, and WebXR's `offsetRay` - a
/// ray in that space's own coordinates, which is the one the app described and [`offset`] already made.
pub fn request(
    session: &XrSession,
    space: &XrReferenceSpace,
    ray: Option<XrRay>,
    slot: Rc<RefCell<Slot>>,
) {
    let options = XrHitTestOptionsInit::new(space);
    if let Some(ray) = &ray {
        options.set_offset_ray(ray);
    }
    let promise: js_sys::Promise = session.request_hit_test_source(&options);
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(source) = JsFuture::from(promise).await
            && let Ok(source) = source.dyn_into::<XrHitTestSource>()
        {
            slot.borrow_mut().fill(source);
        }
    });
}

/// The core's ray as the browser's `XRRay`.
///
/// The two halves are a `DOMPointInit` and an `XRRayDirectionInit`, which is exactly the constructor for an
/// origin and a direction. `XRRay` belongs to the hit test module along with the source, so a browser with one
/// has the other; one that somehow does not is `Unsupported` rather than a source asked for along the space's
/// aim, because the app named a ray and a hit test along a different one is not an answer to it.
pub fn offset(ray: wxr::Ray) -> Result<XrRay, wxr::Error> {
    let origin = DomPointInit::new();
    origin.set_x(f64::from(ray.origin.x));
    origin.set_y(f64::from(ray.origin.y));
    origin.set_z(f64::from(ray.origin.z));
    let direction = XrRayDirectionInit::new();
    direction.set_x(f64::from(ray.direction.x));
    direction.set_y(f64::from(ray.direction.y));
    direction.set_z(f64::from(ray.direction.z));
    XrRay::new_with_dom_point_init_and_direction(&origin, &direction)
        .map_err(|_| wxr::Error::Unsupported("a hit-test ray: this browser has no XRRay".into()))
}

/// Read this frame's hits along `source`, expressed in `base`, into the core's shape.
pub fn results(
    frame: &XrFrame,
    source: &XrHitTestSource,
    base: &XrReferenceSpace,
    out: &mut Vec<wxr::Hit>,
) {
    // One result per surface the ray meets, and an empty sequence for a ray that meets nothing.
    for result in frame.get_hit_test_results(source).iter() {
        let Ok(result) = result.dyn_into::<XrHitTestResult>() else {
            continue;
        };
        // Where is the whole of what a hit test says. A result the runtime will not answer for is one that
        // missed this frame, which is an answer too.
        if let Some(pose) = result.get_pose(base) {
            out.push(wxr::Hit {
                pose: crate::transform(pose.transform()),
            });
        }
    }
}
