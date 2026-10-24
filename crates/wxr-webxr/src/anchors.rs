//! Anchors: places the runtime keeps fixed in the room.
//!
//! The whole module is generated - the Anchors specification's IDL is part of the snapshot - so what is left
//! here is the slot an asynchronously created anchor waits in, and the one call whose throw the specification
//! states in prose rather than in its IDL.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::sys::{XrAnchor, XrFrame, XrReferenceSpace, XrRigidTransform};
use crate::throws;

/// An anchor that has been asked for, and the runtime's answer once it arrives.
#[derive(Default)]
pub struct Slot {
    anchor: Option<XrAnchor>,
    /// Set by [`Slot::forget`], so an answer arriving afterwards is deleted rather than kept. An anchor let
    /// go while its creation was still a promise would otherwise come back from having been released, and
    /// stay tracked by the browser for the rest of the session with no handle left to delete it.
    cancelled: bool,
}

impl Slot {
    /// The anchor, once the runtime has made one.
    pub fn anchor(&self) -> Option<XrAnchor> {
        self.anchor.clone()
    }

    /// Forget it, which is what asking the runtime to delete one leaves behind.
    pub fn forget(&mut self) {
        self.cancelled = true;
        if let Some(anchor) = self.anchor.take() {
            anchor.delete();
        }
    }

    /// Take the browser's answer, unless the anchor was let go while it was still being made - in which case
    /// it is deleted here rather than kept, since no caller will ever ask for it.
    pub fn fill(&mut self, anchor: XrAnchor) {
        if self.cancelled {
            anchor.delete();
        } else {
            self.anchor = Some(anchor);
        }
    }
}

/// Ask the frame for an anchor at `pose` in `space`, and fill the slot when the promise answers.
///
/// The request itself can throw before it is a promise - a frame that is no longer active, or a session the
/// browser did not make with anchors - so this is `Err` for the caller to turn into the core's answer rather
/// than an exception through the backend. A rejection after that is a request that will never fill its slot,
/// which is the same shape as an anchor the runtime lost.
pub fn create(
    frame: &XrFrame,
    space: &XrReferenceSpace,
    pose: XrRigidTransform,
    slot: Rc<RefCell<Slot>>,
) -> Result<(), JsValue> {
    let promise = throws::create_anchor(frame, &pose, space)?;
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(anchor) = JsFuture::from(promise).await
            && let Ok(anchor) = anchor.dyn_into::<XrAnchor>()
        {
            slot.borrow_mut().fill(anchor);
        }
    });
    Ok(())
}

/// Where the anchor is now, in `space`, or `None` when the runtime has lost it.
pub fn pose(frame: &XrFrame, anchor: &XrAnchor, base: &XrReferenceSpace) -> Option<wxr::Pose> {
    // Caught: an anchor the runtime has dropped throws from `anchorSpace`, and a place that is gone is a `None`
    // rather than a frame to fail.
    let space = throws::anchor_space(anchor).ok()?;
    let pose = frame.get_pose(&space, base)?;
    Some(crate::transform(pose.transform()))
}
