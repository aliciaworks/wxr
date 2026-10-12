//! Places the runtime keeps fixed, which is `XR_FB_spatial_entity` driven by hand.
//!
//! OpenXR has two anchor APIs and this one is the one that fits the core. An anchor here is an `XrSpace`,
//! which is already what [`wxr::Session::anchor_pose`] asks for when it asks where one is now - and the
//! Khronos `XR_EXT_spatial_anchor` is not: it produces a *spatial entity*, which needs a spatial context
//! made asynchronously with a capability configuration, and a pose comes back through a snapshot of its
//! components. That is the right shape for a runtime whose world is entities all the way down, and it is
//! three more state machines than the core's three methods ask for.
//!
//! The cost of the simpler one is that it is Meta's: a runtime without `XR_FB_spatial_entity` has no
//! anchors from this backend, and the honest answer then is [`wxr::Error::Unsupported`] rather than a
//! handle that never resolves.
//!
//! The other thing this file is careful about is what an event's `space` means when the request failed.
//! The spec does not say - there is no valid-usage block on the structure and the one sentence about
//! ownership is written for the success case - so the handle is taken ownership of only when the runtime
//! said it succeeded, and a failed request is a slot that stays empty.

use super::*;

/// What one slot of the anchor table is.
pub(super) enum Slot {
    /// The request was made and the runtime has not posted the completion event yet, and this is the id it
    /// will arrive under. What the core's [`wxr::Session::anchor_pose`] answers `None` for - a place that is
    /// not there *yet*, which is not the same as one that is gone.
    Pending(openxr_sys::AsyncRequestIdFB),
    /// The runtime made it. This is the space, and it is this backend that owns it now.
    Live(xr::Space),
}

/// The anchors this session has, and the requests in flight.
pub(super) struct Anchors {
    table: xr::raw::SpatialEntityFB,
    /// The session, kept because the completion event hands back a *raw* handle and an `xr::Space` is made
    /// from one plus the session it belongs to. It is an `Arc` underneath, so this keeps the session alive
    /// rather than borrowing it.
    session: xr::Session<xr::Vulkan>,
    /// By the core's own id, which is the index. A release leaves a hole rather than shifting the ones
    /// after it, because a caller holding an id has to keep holding the same one.
    slots: Vec<Option<Slot>>,
}

impl Anchors {
    /// Turn the spatial-entity system on, or say this runtime has no way to keep a place fixed.
    pub(super) fn new(session: &xr::Session<xr::Vulkan>, enabled: bool) -> Option<Self> {
        if !enabled {
            log::info!("wxr-openxr: no anchors: this runtime has no XR_FB_spatial_entity");
            return None;
        }
        let Some(table) = session.instance().exts().fb_spatial_entity.as_ref() else {
            log::info!("wxr-openxr: no anchors: the extension was enabled but did not load");
            return None;
        };
        Some(Self {
            table: *table,
            session: session.clone(),
            slots: Vec::new(),
        })
    }

    /// Take the completion of a request, which is the only way an anchor ever becomes real.
    ///
    /// Called from the event loop, and it reports nothing back to the caller: the core's shape for "the
    /// anchor you asked for is not ready" is the `None` [`wxr::Session::anchor_pose`] already answers.
    pub(super) fn complete(
        &mut self,
        request: openxr_sys::AsyncRequestIdFB,
        result: openxr_sys::Result,
        space: openxr_sys::Space,
    ) {
        // Matched by the id rather than by "the oldest one pending", because a runtime may answer out of
        // order and an anchor that took over another's slot would be a caller's handle meaning something
        // else entirely - which is the one kind of wrong that does not show up as a failure.
        let Some(index) = self
            .slots
            .iter()
            .position(|slot| matches!(slot, Some(Slot::Pending(id)) if *id == request))
        else {
            // A completion for a request this backend has already given up on, which a runtime does after a
            // session has been torn down. Nothing to do and nothing to warn about.
            return;
        };
        let slot = &mut self.slots[index];
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: the runtime would not make an anchor: {result:?}");
            *slot = None;
            return;
        }
        // SAFETY: the runtime says this handle is a newly created space belonging to this session, and the
        // `Arc` above is that session. The id is the one the slot was filed under, so what is written here
        // is the very request the completion names - which is the difference between "the anchor" and "an
        // anchor" when the runtime answers out of order.
        *slot = Some(Slot::Live(unsafe {
            xr::Space::reference_from_raw(self.session.clone(), space)
        }));
    }
}

impl OpenXrSession {
    /// Ask the runtime to keep a place fixed, and name it before the runtime has answered.
    ///
    /// The id comes back now and the anchor exists when the completion event arrives, which is a frame or
    /// two later - the same shape a hit-test source has, and for the same reason: the runtime is the one
    /// that knows when it has found the place.
    pub(super) fn anchor_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        pose: wxr::Pose,
    ) -> Result<wxr::Anchor, wxr::Error> {
        let Some(anchors) = self.anchors.as_mut() else {
            return Err(wxr::Error::Unsupported(
                "anchors: this runtime has no XR_FB_spatial_entity".into(),
            ));
        };
        let Some(base) = self.spaces.get(space.id() as usize) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let info = openxr_sys::SpatialAnchorCreateInfoFB {
            ty: openxr_sys::StructureType::SPATIAL_ANCHOR_CREATE_INFO_FB,
            next: std::ptr::null(),
            space: base.as_raw(),
            pose_in_space: crate::convert::posef(pose),
            time: self.predicted,
        };
        let mut request: openxr_sys::AsyncRequestIdFB = Default::default();
        // SAFETY: the session and the table are the runtime's own, and `info` and `request` outlive the
        // call. The slot is pushed only once the runtime has taken the request, because an id for a request
        // that was refused is an id that will never be answered.
        let result = unsafe {
            (anchors.table.create_spatial_anchor)(self.session.as_raw(), &info, &mut request)
        };
        if result != openxr_sys::Result::SUCCESS {
            return Err(wxr::Error::Unsupported(format!(
                "anchors: the runtime would not take one: {result:?}"
            )));
        }
        let id = anchors.slots.len() as u32;
        anchors.slots.push(Some(Slot::Pending(request)));
        Ok(wxr::Anchor::new(id))
    }

    /// Where an anchor is now, in the space given.
    ///
    /// `None` is three things and the caller cannot tell them apart, which is what
    /// [`wxr::Features::ANCHORS`] is for: one the runtime has not answered for yet, one it has lost, and one
    /// that was released. The spec is explicit that a fresh anchor is locatable the moment it exists, so the
    /// middle one is a real answer rather than this backend giving up.
    pub(super) fn anchor_pose_impl(
        &mut self,
        anchor: wxr::Anchor,
        space: wxr::ReferenceSpace,
    ) -> Result<Option<wxr::Pose>, wxr::Error> {
        let Some(anchors) = self.anchors.as_ref() else {
            return Err(wxr::Error::Unsupported(
                "anchors: this runtime has no XR_FB_spatial_entity".into(),
            ));
        };
        let Some(base) = self.spaces.get(space.id() as usize) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let Some(Slot::Live(found)) = anchors
            .slots
            .get(anchor.id() as usize)
            .and_then(|s| s.as_ref())
        else {
            return Ok(None);
        };
        match found.locate(base, self.predicted) {
            // A place the runtime will not stand behind the position of is a place it has lost, which is
            // what the core says to answer rather than a pose that is a guess.
            Ok(location) if is_located(location.location_flags) => {
                Ok(Some(crate::convert::pose(location.pose)))
            }
            Ok(_) => Ok(None),
            Err(error) => {
                log::debug!("wxr-openxr: locating an anchor: {error:?}");
                Ok(None)
            }
        }
    }

    /// Tell the runtime the app is done with one.
    pub(super) fn release_anchor_impl(&mut self, anchor: wxr::Anchor) {
        let Some(anchors) = self.anchors.as_mut() else {
            return;
        };
        // Taking the slot and letting the space fall out of scope is the whole of it: dropping an `xr::Space`
        // is `xrDestroySpace`, and OpenXR has no separate destroy for an anchor because there is nothing
        // else it could be.
        if let Some(slot) = anchors.slots.get_mut(anchor.id() as usize) {
            *slot = None;
        }
    }
}
