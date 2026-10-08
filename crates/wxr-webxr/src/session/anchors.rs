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
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
            return Err(wxr::Error::Unavailable("a frame has not begun yet".into()));
        };
        let Some(base) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let slot = Rc::new(RefCell::new(crate::anchors::Slot::default()));
        crate::anchors::create(&frame, &base, rigid(pose)?, slot.clone());
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
        let Some(anchor) = self
            .anchors
            .get(anchor.id() as usize)
            .and_then(|slot| slot.borrow().anchor())
        else {
            return Ok(None);
        };
        let Some(base) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Ok(None);
        };
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
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
        if let Some(anchor) = slot.borrow().anchor() {
            anchor.delete();
        }
        slot.borrow_mut().forget();
    }
}
