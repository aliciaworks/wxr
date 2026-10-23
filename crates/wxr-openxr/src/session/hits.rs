//! Where a ray out of a space meets the world, which is `XR_META_environment_raycast` driven by hand.
//!
//! The shape this extension has is not the shape the core's vocabulary has, and the difference is worth
//! being exact about. WebXR's hit-test source *is* the ray: an app requests one with an origin and a
//! direction and the runtime keeps it. OpenXR's raycaster has no ray at all - `XrEnvironmentRaycasterCreate
//! InfoMETA` is a type and a chain and nothing else - and the ray is an input to every cast. So a source
//! here is the space the ray is measured out of, and the ray is the space's own `-Z`, which is exactly the
//! ray `wxr::Session::hit_test_source` says it asks for. One raycaster serves every source, which is why it
//! is made once and lazily: making it is asynchronous, and an app that never asks for a hit never pays for
//! one.
//!
//! Making it is also why this needs `XR_EXT_future` and why a runtime without that extension has no
//! raycaster even if it lists this one - `xrCreateInstance` refuses the pair, which is the one place the
//! dependency shows up. That is decided in `lib.rs` and only the state of it is here.

use super::*;

/// What one slot of the source table is: the space a source's ray is measured out of, by its index into the
/// session's spaces, or nothing once it was released or never made.
type Source = Option<usize>;

/// The raycaster, and the sources measured out of it.
pub(super) struct Raycaster {
    table: xr::raw::EnvironmentRaycastMETA,
    /// The future half, which is a separate extension doing the same job for every async operation in
    /// OpenXR. Kept because the raycaster is made through it and the two are enabled together.
    futures: xr::raw::FutureEXT,
    /// The session, for the two calls that take one.
    session: xr::Session<xr::Vulkan>,
    /// While the raycaster is being made, and gone once it is.
    making: Option<openxr_sys::FutureEXT>,
    /// The raycaster itself. `None` until the future has been polled to readiness and completed.
    handle: Option<openxr_sys::EnvironmentRaycasterMETA>,
    /// By the core's own id, which is the index.
    sources: Vec<Source>,
}

impl Raycaster {
    /// Get the pair of extensions this needs, or say the runtime has no way to ask where a ray lands.
    pub(super) fn new(session: &xr::Session<xr::Vulkan>, enabled: bool) -> Option<Self> {
        if !enabled {
            log::info!(
                "wxr-openxr: no hit testing: this runtime has no XR_META_environment_raycast with \
                 XR_EXT_future"
            );
            return None;
        }
        let exts = session.instance().exts();
        let (Some(table), Some(futures)) = (
            exts.meta_environment_raycast.as_ref(),
            exts.ext_future.as_ref(),
        ) else {
            log::info!("wxr-openxr: no hit testing: the extensions were enabled but did not load");
            return None;
        };
        Some(Self {
            table: *table,
            futures: *futures,
            session: session.clone(),
            making: None,
            handle: None,
            sources: Vec::new(),
        })
    }

    /// Start making the raycaster if nothing has yet.
    ///
    /// Called from the first source that is asked for, because that is the first evidence that an app wants
    /// one at all - and the cost is a frame or two of nothing, which is the same shape the core describes.
    fn start(&mut self) {
        if self.making.is_some() || self.handle.is_some() {
            return;
        }
        let info = openxr_sys::EnvironmentRaycasterCreateInfoMETA {
            ty: openxr_sys::StructureType::ENVIRONMENT_RAYCASTER_CREATE_INFO_META,
            // The whole of the create info. There is no ray in it, which is the thing this file's module
            // note is about - the ray arrives with every cast.
            next: std::ptr::null(),
        };
        let mut future = null_future();
        // SAFETY: the session, the table and the info are live and outlive the call, and `future` is the
        // out-parameter the runtime writes the handle it will complete on into.
        let result = unsafe {
            (self.table.create_environment_raycaster_async)(
                self.session.as_raw(),
                &info,
                &mut future,
            )
        };
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: starting to make a raycaster: {result:?}");
            return;
        }
        self.making = Some(future);
    }

    /// Move the making of the raycaster along, once a frame.
    pub(super) fn poll(&mut self) {
        let Some(future) = self.making else { return };
        let poll_info = openxr_sys::FuturePollInfoEXT {
            ty: openxr_sys::StructureType::FUTURE_POLL_INFO_EXT,
            next: std::ptr::null(),
            future,
        };
        let mut poll_result = openxr_sys::FuturePollResultEXT {
            ty: openxr_sys::StructureType::FUTURE_POLL_RESULT_EXT,
            next: std::ptr::null_mut(),
            state: openxr_sys::FutureStateEXT::PENDING,
        };
        // SAFETY: the instance this session came from is live, the poll info outlives the call, and the
        // result is an out-structure with its header filled in.
        let result = unsafe {
            (self.futures.poll_future)(
                self.session.instance().as_raw(),
                &poll_info,
                &mut poll_result,
            )
        };
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: polling the raycaster's future: {result:?}");
            // The future is not ready and never will be through this path. Cancelling it is what tells the
            // runtime to stop making the raycaster, rather than leaving a handle with no owner and a
            // completion that nobody will collect.
            if let Some(future) = self.making.take() {
                self.cancel(future);
            }
            return;
        }
        if poll_result.state != openxr_sys::FutureStateEXT::READY {
            return;
        }
        // A future that is ready is finished, and it has to be completed exactly once - so it is taken here
        // and the completion below cannot be reached twice with the same one.
        let future = self.making.take().expect("checked above");
        let mut completion = openxr_sys::EnvironmentRaycasterCreateCompletionMETA {
            ty: openxr_sys::StructureType::ENVIRONMENT_RAYCASTER_CREATE_COMPLETION_META,
            next: std::ptr::null_mut(),
            future_result: openxr_sys::Result::SUCCESS,
            environment_raycaster: null_raycaster(),
        };
        // SAFETY: the session is live, the future is in the READY state and has not been completed, and the
        // completion is an out-structure with its header filled in.
        let result = unsafe {
            (self.table.create_environment_raycaster_complete)(
                self.session.as_raw(),
                future,
                &mut completion,
            )
        };
        // The result of the *call* says whether it was valid to make it; the async operation's own result is
        // in `future_result`, and the spec is explicit that the two are not the same thing.
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: completing the raycaster: {result:?}");
            return;
        }
        if completion.future_result != openxr_sys::Result::SUCCESS {
            log::warn!(
                "wxr-openxr: the runtime would not make a raycaster: {:?}",
                completion.future_result
            );
            return;
        }
        self.handle = Some(completion.environment_raycaster);
        log::debug!("wxr-openxr: hit testing is ready");
    }

    /// Tell the runtime to stop making what a pending future is making.
    ///
    /// A future that is dropped is a future the runtime is still working on: `XR_EXT_future` gives no handle
    /// to destroy, and the operation's own completion is the only thing that would collect it. Cancelling is
    /// the last call the spec has for one, and it is safe to make on a future the runtime has already
    /// finished or invalidated - the answer is an error, not a second destruction.
    fn cancel(&self, future: openxr_sys::FutureEXT) {
        let info = openxr_sys::FutureCancelInfoEXT {
            ty: openxr_sys::StructureType::FUTURE_CANCEL_INFO_EXT,
            next: std::ptr::null(),
            future,
        };
        // SAFETY: the instance is live, and the cancel info outlives the call. The future came from this
        // instance's raycaster create and has been completed, cancelled, or is this frame's pending one.
        let result =
            unsafe { (self.futures.cancel_future)(self.session.instance().as_raw(), &info) };
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: cancelling the raycaster's future: {result:?}");
        }
    }

    /// Where one ray lands, or nothing.
    fn cast(
        &self,
        base_space: openxr_sys::Space,
        time: xr::Time,
        origin: openxr_sys::Vector3f,
        direction: openxr_sys::Vector3f,
    ) -> Option<openxr_sys::Posef> {
        let raycaster = self.handle?;
        let info = openxr_sys::EnvironmentRaycastHitGetInfoMETA {
            ty: openxr_sys::StructureType::ENVIRONMENT_RAYCAST_HIT_GET_INFO_META,
            next: std::ptr::null(),
            base_space,
            time,
            origin,
            direction,
            // No filter, which is what the core's ray is: an unbounded one out of the space's own `-Z`. A
            // distance filter is the one thing this extension defines, and the core has nowhere to carry a
            // limit from.
            filter_count: 0,
            filters: std::ptr::null(),
        };
        let mut hit = openxr_sys::EnvironmentRaycastHitMETA {
            ty: openxr_sys::StructureType::ENVIRONMENT_RAYCAST_HIT_META,
            next: std::ptr::null(),
            status: openxr_sys::EnvironmentRaycastHitStatusMETA::NO_HIT,
            pose: xr::Posef::IDENTITY,
        };
        // SAFETY: the raycaster is live, `info` outlives the call, and `hit` is an out-structure whose
        // header is filled in and whose `next` the extension requires to be null.
        let result =
            unsafe { (self.table.perform_environment_raycast)(raycaster, &info, &mut hit) };
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: casting a ray: {result:?}");
            return None;
        }
        match hit.status {
            // `HIT_POINT_OCCLUDED` is a ray that met the world through something else - a wall behind a
            // screen - and the spec says its pose is still the best estimate of where it landed, which is
            // what a placement wants. `HIT_INVALID_ORIENTATION` has a position and undefined orientation,
            // and a pose half of which is a guess is a pose that would be believed.
            status
                if status == openxr_sys::EnvironmentRaycastHitStatusMETA::HIT
                    || status
                        == openxr_sys::EnvironmentRaycastHitStatusMETA::HIT_POINT_OCCLUDED =>
            {
                Some(hit.pose)
            }
            _ => None,
        }
    }
}

impl OpenXrSession {
    /// Ask for a source of hits along a space's own aim.
    pub(super) fn hit_test_source_impl(
        &mut self,
        space: wxr::ReferenceSpace,
    ) -> Result<wxr::HitTestSource, wxr::Error> {
        let Some(raycast) = self.raycast.as_mut() else {
            return Err(wxr::Error::Unsupported(
                "hit testing: this runtime has no XR_META_environment_raycast".into(),
            ));
        };
        if self.spaces.get(space.id() as usize).is_none() {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        // The first source is what makes the raycaster, and then it is a frame or two before any of them
        // answer - which is the core's "a frame or two with no source" rather than a silent failure.
        raycast.start();
        let id = raycast.sources.len() as u32;
        raycast.sources.push(Some(space.id() as usize));
        Ok(wxr::HitTestSource::new(id))
    }

    /// Let a source of hits go: the entry that says which space the ray is measured from.
    pub(super) fn release_hit_test_source_impl(&mut self, source: wxr::HitTestSource) {
        let Some(raycast) = self.raycast.as_mut() else {
            return;
        };
        // Taken in place rather than taken out of the list, because the id *is* the position - pulling one
        // out would hand every source after it somebody else's ray, and the caller's handle would still go
        // on saying its own number. OpenXR has nothing to undo here either: the raycaster is session-wide
        // and has no per-source object, so this emptied entry is all of it, and what a caller sees is
        // `hits` coming back empty because this session says so.
        if let Some(slot) = raycast.sources.get_mut(source.id() as usize) {
            *slot = None;
        }
    }

    /// Where one source's ray landed this frame, in the space given.
    pub(super) fn hits_impl(
        &mut self,
        source: wxr::HitTestSource,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Hit>,
    ) -> Result<(), wxr::Error> {
        out.clear();
        let Some(raycast) = self.raycast.as_ref() else {
            return Err(wxr::Error::Unsupported(
                "hit testing: this runtime has no XR_META_environment_raycast".into(),
            ));
        };
        let Some(base) = self.spaces.get(space.id() as usize) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        // A source whose space was released, or the raycaster still being made: an empty list, which is
        // what "the ray has not been cast yet" looks like to a caller.
        let Some(Some(from)) = raycast.sources.get(source.id() as usize).copied() else {
            return Ok(());
        };
        let Some(aim) = self.spaces.get(from) else {
            return Ok(());
        };
        let Ok(location) = aim.locate(base, self.predicted) else {
            return Ok(());
        };
        // A ray is an origin and a direction, and each half has its own bit. A source the runtime has not
        // actively tracked in both is a source whose ray would be cast from the last place it was seen, or
        // from the identity - and either is a hit at a place the user is no longer pointing.
        let flags = location.location_flags;
        if !flags.contains(xr::SpaceLocationFlags::POSITION_TRACKED)
            || !flags.contains(xr::SpaceLocationFlags::ORIENTATION_TRACKED)
        {
            return Ok(());
        }
        // The ray is out of the source space's own `-Z`, which is the aim every space in OpenXR has and the
        // ray the core says it asks for. The pose is in `base`, which is what the hit has to come back in
        // too - so the ray is built in `base` rather than cast in one frame and moved into another.
        let aim = crate::convert::pose(location.pose);
        let direction = (aim.orientation * wxr::glam::Vec3::NEG_Z).normalize();
        let Some(found) = raycast.cast(
            base.as_raw(),
            self.predicted,
            vector(aim.position),
            vector(direction),
        ) else {
            return Ok(());
        };
        // The hit is in the space the ray was cast in, and OpenXR puts the surface's normal on the hit
        // pose's `+Z` - "rotating {0, 0, 1} by this orientation" - which is the one convention the core
        // turns into `+Y`. See `convert::surface`.
        // One, and the extension is why: `xrPerformEnvironmentRaycastMETA` answers with *the* hit, and a
        // second call with the same ray would be the same answer. WebXR's list is a list because a browser
        // can hand back several surfaces along one ray - this is one surface, so `out` is left with what it
        // met and nothing to pad it with.
        out.push(wxr::Hit {
            pose: crate::convert::surface(crate::convert::pose(found)),
        });
        Ok(())
    }
}

/// A glam vector as OpenXR's, which is the shape every ray parameter arrives in.
fn vector(v: wxr::glam::Vec3) -> openxr_sys::Vector3f {
    openxr_sys::Vector3f {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

/// A null future, which is what the out-parameter of an async start begins as.
fn null_future() -> openxr_sys::FutureEXT {
    // SAFETY: this is the null handle, and nothing dereferences it until the runtime has written one.
    unsafe { std::mem::zeroed() }
}

/// A null raycaster, which is what the out-parameter of the completion begins as.
fn null_raycaster() -> openxr_sys::EnvironmentRaycasterMETA {
    // SAFETY: this is the null handle, and nothing dereferences it until the runtime has written one.
    unsafe { std::mem::zeroed() }
}

impl Drop for Raycaster {
    fn drop(&mut self) {
        // A create still in flight has no raycaster to destroy yet, and dropping the future would leave the
        // runtime making one that nobody collects. Cancel it first; the completion the runtime would have
        // delivered is then not delivered, which is what cancellation means.
        if let Some(future) = self.making.take() {
            self.cancel(future);
        }
        if let Some(handle) = self.handle.take() {
            // SAFETY: the raycaster is this session's and is being destroyed exactly once.
            let result = unsafe { (self.table.destroy_environment_raycaster)(handle) };
            if result != openxr_sys::Result::SUCCESS {
                log::debug!("wxr-openxr: destroying the raycaster: {result:?}");
            }
        }
    }
}
