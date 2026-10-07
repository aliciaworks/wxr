//! Hit testing: where a ray out of a space meets the world.
//!
//! `XRHitTestSource` is created asynchronously and is otherwise opaque, so one waits in a slot until the
//! browser answers - and a frame with no source yet is a frame with no hits rather than a frame to fail.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::sys::{
    XrFrame, XrHitTestOptionsInit, XrHitTestResult, XrHitTestSource, XrReferenceSpace, XrSession,
};

/// A source that has been asked for, and the browser's answer once it arrives.
#[derive(Default)]
pub struct Slot {
    source: Option<XrHitTestSource>,
}

impl Slot {
    /// The source, if the browser has answered.
    pub fn source(&self) -> Option<XrHitTestSource> {
        self.source.clone()
    }
}

/// Ask the browser for a source aiming out of `space`, and fill the slot when it answers.
///
/// The only option is the space: everything else about a hit test source has a default, and a ray is a ray.
pub fn request(session: &XrSession, space: &XrReferenceSpace, slot: Rc<RefCell<Slot>>) {
    let options = XrHitTestOptionsInit::new(space);
    let promise: js_sys::Promise = session.request_hit_test_source(&options);
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(source) = JsFuture::from(promise).await
            && let Ok(source) = source.dyn_into::<XrHitTestSource>()
        {
            slot.borrow_mut().source = Some(source);
        }
    });
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
