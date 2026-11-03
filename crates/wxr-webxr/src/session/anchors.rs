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
        // An anchor is a feature, and a session that did not ask for `anchors` cannot make one. Say so here,
        // rather than returning a handle for a request the browser will only reject: a handle that never
        // answers is the one shape this backend already refuses for every other asynchronous resource.
        let Some(session) = self.inner.borrow().session.clone() else {
            return Err(wxr::Error::Unavailable(
                "the session has not started yet".into(),
            ));
        };
        if !has_feature(&session, "anchors") {
            return Err(wxr::Error::Unsupported("anchors".into()));
        }
        // The space before the frame, as the other two backends do: `createAnchor` needs a real space and a
        // real frame, and a handle this session never handed out is refused as such whether or not a frame
        // happens to be running.
        let Some(base) = self.resolved_space(space)? else {
            return Err(wxr::Error::Unavailable(
                "the space has not resolved yet".into(),
            ));
        };
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
    pub(super) fn release_anchor_impl(&mut self, anchor: wxr::Anchor) {
        let Some(slot) = self.anchors.get(anchor.id() as usize) else {
            return;
        };
        // `forget` deletes an anchor that has arrived and remembers that one which has not must be deleted
        // when it does - so this is the whole of a release whether or not the promise has answered.
        slot.borrow_mut().forget();
    }
}
