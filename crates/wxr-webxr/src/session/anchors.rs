//! The anchors this session made.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here.

use super::*;

impl WebXrSession {
    pub(super) fn anchor_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        pose: wxr::Pose,
    ) -> Result<wxr::Anchor, wxr::Error> {
        // The space before the frame, as the other two backends do: `createAnchor` needs a real space and a
        // real frame, and a handle this session never handed out is refused as such whether or not a frame
        // happens to be running - and before the feature, so the answer does not depend on what this session
        // asked the browser for.
        let Some(base) = self.resolved_space(space)? else {
            return Err(wxr::Error::Unavailable(
                "the space has not resolved yet".into(),
            ));
        };
        let Some(session) = self.inner.borrow().session.clone() else {
            return Err(wxr::Error::Unavailable(
                "the session has not started yet".into(),
            ));
        };
        if !has_feature(&session, "anchors") {
            return Err(wxr::Error::Unsupported("anchors".into()));
        }
        let Some(frame) = self.frame() else {
            return Err(wxr::Error::Unavailable("a frame has not begun yet".into()));
        };
        let slot = Rc::new(RefCell::new(crate::anchors::Slot::default()));
        crate::anchors::create(&frame, &base, rigid(pose)?, slot.clone())
            .map_err(|error| wxr::Error::Rejected(format!("{error:?}")))?;
        self.anchors.push(slot);
        Ok(wxr::Anchor::new((self.anchors.len() - 1) as u32))
    }
}

impl WebXrSession {
    pub(super) fn anchor_pose_impl(
        &mut self,
        anchor: wxr::Anchor,
        space: wxr::ReferenceSpace,
    ) -> Result<Option<wxr::Pose>, wxr::Error> {
        // The space first, as the other two backends answer: a handle this session never handed out is not a
        // place the anchor was lost from, and `Ok(None)` would say it was.
        let Some(base) = self.resolved_space(space)? else {
            return Ok(None);
        };
        let Some(anchor) = self
            .anchors
            .get(anchor.id() as usize)
            .and_then(|slot| slot.borrow().anchor())
        else {
            return Ok(None);
        };
        let Some(frame) = self.frame() else {
            return Ok(None);
        };
        Ok(crate::anchors::pose(&frame, &anchor, &base))
    }
}

impl WebXrSession {
    /// The browser's name for an anchor, asked for the first time this is called.
    ///
    /// An anchor the frame has not made yet has no name to ask for, and the answer is `None` for the same
    /// reason `anchor_pose` is: there is nothing there yet. The ask is made once, and a browser that refuses
    /// leaves the answer `None` rather than asking again every frame.
    pub(super) fn anchor_id_impl(&mut self, anchor: wxr::Anchor) -> Option<wxr::AnchorId> {
        let slot = self.anchors.get(anchor.id() as usize)?.clone();
        {
            let slot = slot.borrow();
            if let Some(id) = slot.id() {
                return Some(id);
            }
            if slot.asked() {
                return None;
            }
        }
        let anchor = slot.borrow().anchor()?;
        slot.borrow_mut().ask();
        crate::anchors::request_handle(&anchor, slot);
        None
    }
}

impl WebXrSession {
    pub(super) fn release_anchor_impl(&mut self, anchor: wxr::Anchor) {
        let Some(slot) = self.anchors.get(anchor.id() as usize) else {
            return;
        };
        // `forget` deletes an anchor that has arrived and remembers that one which has not must be deleted
        // when it does - so this is the whole of a release whether or not the promise has answered.
        slot.borrow_mut().forget();
    }
}
