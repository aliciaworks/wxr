//! The names an anchor keeps after the session ends: `XR_META_spatial_entity_persistence` and
//! `XR_META_spatial_entity_discovery`.
//!
//! An anchor is an `XrSpace`, and a space does not outlive the session that made it. What outlives it is the
//! UUID the runtime gave the entity - [`wxr::AnchorId`] - so keeping an anchor between runs is three asks
//! that all have to be answered before the name means anything:
//!
//! - **Store it.** A freshly made anchor is only `LOCATABLE`; saving needs `STORABLE`, which is a component
//!   status change (`xrSetSpaceComponentStatusFB`), and only then does `xrSaveSpacesMETA` have anything to
//!   keep. Both are asynchronous, so the name is answered after two events rather than one call.
//! - **Find it again.** `xrDiscoverSpacesMETA` under a UUID filter loads the stored entity and answers with a
//!   *new* `XrSpace`; `xrRetrieveSpaceDiscoveryResultsMETA` is the two-call idiom that takes it out.
//! - **Forget it.** `xrEraseSpacesMETA` takes the UUIDs themselves, so erasing a name needs no lookup and no
//!   space - which is why [`wxr::Session::delete_persistent_anchor`] is one request even though the other two
//!   are round trips.
//!
//! # Why this generation and not the `XR_FB_*` one
//!
//! `XR_FB_spatial_entity_query` and `XR_FB_spatial_entity_storage` would do the same jobs, and the registry
//! marks both **obsolete** in favour of these two: `xrSaveSpaceFB`/`xrEraseSpaceFB` take a space, which for
//! an erase means the caller has to find it first. The one thing still taken from `XR_FB_spatial_entity` is
//! the component status, because `STORABLE` is defined there and the META extension assumes it.
//!
//! # What is answered when
//!
//! Nothing here is reported as an event, because the core has no event for "your anchor is stored now": the
//! same shape [`super::anchors`] has for creation. [`wxr::Session::anchor_id`] answers `None` until the save
//! is confirmed, and [`wxr::Session::restore_anchor`] hands back a handle that `anchor_pose` fills in.

use super::anchors::{Saved, Slot};
use super::*;

/// The two tables that keep a space, and the erase requests waiting for an answer.
pub(super) struct Persistence {
    pub(super) discovery: xr::raw::SpatialEntityDiscoveryMETA,
    storage: xr::raw::SpatialEntityPersistenceMETA,
    /// Erase requests the runtime has not answered. Erasing by name names no space, so nothing else is kept.
    erases: Vec<openxr_sys::AsyncRequestIdFB>,
}

impl Persistence {
    /// Both extensions, or neither: a name that can be written and not read back is not one.
    pub(super) fn new(session: &xr::Session<xr::Vulkan>) -> Option<Self> {
        let extensions = session.instance().exts();
        let Some(discovery) = extensions.meta_spatial_entity_discovery.as_ref() else {
            log::info!(
                "wxr-openxr: no anchor names: this runtime has no XR_META_spatial_entity_discovery"
            );
            return None;
        };
        let Some(storage) = extensions.meta_spatial_entity_persistence.as_ref() else {
            log::info!(
                "wxr-openxr: no anchor names: this runtime has no XR_META_spatial_entity_persistence"
            );
            return None;
        };
        Some(Self {
            discovery: *discovery,
            storage: *storage,
            erases: Vec::new(),
        })
    }
}

/// Why a session cannot answer a question about names: an anchor is `XR_FB_spatial_entity` and its name is
/// the META pair, so a runtime missing either half keeps anchors for one session only.
pub(super) fn unsupported(what: &str) -> wxr::Error {
    wxr::Error::Unsupported(
        format!("{what}: this runtime does not keep anchors between sessions").into(),
    )
}

impl Anchors {
    /// The runtime's name for a live anchor, asking it to keep the place first if it has not been asked.
    ///
    /// The UUID is known from the moment the anchor exists, but one the runtime is not storing finds nothing
    /// in the next run - so what this answers is the name *once it is worth writing down*, and the first call
    /// is the one that starts the save. A caller polling gets it a few frames later, the shape WebXR's
    /// `requestPersistentHandle` promise has.
    pub(super) fn name(&mut self, anchor: wxr::Anchor) -> Option<wxr::AnchorId> {
        let index = anchor.id() as usize;
        // Read what the slot says and let the borrow go: asking the runtime is a second borrow of this
        // table, and the answer is written back into the slot by the completion, not by this call.
        let space = match self.slots.get(index).and_then(Option::as_ref)? {
            Slot::Live {
                id,
                saved: Saved::Stored,
                ..
            } => return Some(*id),
            Slot::Live {
                space,
                saved: Saved::Unstored,
                ..
            } => space.as_raw(),
            // A save the runtime has not answered is not a name yet, and one it says it cannot keep is one
            // this session will never have.
            Slot::Live { .. } | Slot::Pending(_) | Slot::Restoring { .. } => return None,
        };
        self.ask_storable(index, space);
        None
    }

    /// Enable `STORABLE` on a live entity, which is the component every save needs.
    ///
    /// The change is asynchronous and its completion calls [`Anchors::save`]. Already enabled means the
    /// save is next, a component that is not supported is final, and one already pending asks again.
    fn ask_storable(&mut self, index: usize, space: openxr_sys::Space) {
        if self.persistence.is_none() {
            self.saved(index, Saved::Refused);
            return;
        }
        let info = openxr_sys::SpaceComponentStatusSetInfoFB {
            ty: openxr_sys::StructureType::SPACE_COMPONENT_STATUS_SET_INFO_FB,
            next: std::ptr::null(),
            component_type: openxr_sys::SpaceComponentTypeFB::STORABLE,
            enabled: openxr_sys::TRUE,
            // The spec asks for a duration rather than offering "eventually", and infinite is the answer
            // that does not cancel a request that is about to succeed.
            timeout: openxr_sys::Duration::INFINITE,
        };
        let mut request: openxr_sys::AsyncRequestIdFB = Default::default();
        // SAFETY: the table is the runtime's own, `space` is a live space belonging to this session, and
        // `info` and `request` outlive the call.
        let result = unsafe { (self.table.set_space_component_status)(space, &info, &mut request) };
        match result {
            openxr_sys::Result::SUCCESS => self.saved(index, Saved::Enabling(request)),
            // Already on: the save is the next step, not a second enable.
            openxr_sys::Result::ERROR_SPACE_COMPONENT_STATUS_ALREADY_SET_FB => {
                self.save(index, space)
            }
            openxr_sys::Result::ERROR_SPACE_COMPONENT_NOT_SUPPORTED_FB => {
                self.saved(index, Saved::Refused)
            }
            // A change already in flight, or a runtime that will not take the ask this frame: the next
            // `anchor_id` call is the retry, which is what an app polling for a name will do anyway.
            openxr_sys::Result::ERROR_SPACE_COMPONENT_STATUS_PENDING_FB => {}
            _ => log::debug!("wxr-openxr: asking for a storable anchor: {result:?}"),
        }
    }

    /// Ask the runtime to keep a space, now that the component it needs is on.
    ///
    /// A session that is not focused cannot save - the runtime says `XR_ERROR_SESSION_NOT_FOCUSED` - and that
    /// is not a name to give up on: the slot goes back to unstored and the next call asks again.
    fn save(&mut self, index: usize, space: openxr_sys::Space) {
        let Some(persistence) = self.persistence.as_ref() else {
            self.saved(index, Saved::Refused);
            return;
        };
        let mut spaces = [space];
        let info = openxr_sys::SpacesSaveInfoMETA {
            ty: openxr_sys::StructureType::SPACES_SAVE_INFO_META,
            next: std::ptr::null(),
            space_count: 1,
            spaces: spaces.as_mut_ptr(),
        };
        let mut request: openxr_sys::AsyncRequestIdFB = Default::default();
        // SAFETY: the table is the runtime's own, the session is live, and `spaces`, `info` and `request`
        // outlive the call.
        let result = unsafe {
            (persistence.storage.save_spaces)(self.session.as_raw(), &info, &mut request)
        };
        match result {
            openxr_sys::Result::SUCCESS => self.saved(index, Saved::Saving(request)),
            openxr_sys::Result::SESSION_NOT_FOCUSED => {
                log::debug!(
                    "wxr-openxr: an anchor cannot be saved while the session is not focused"
                )
            }
            _ => log::debug!("wxr-openxr: saving an anchor: {result:?}"),
        }
    }

    /// What a space component status change turned out to be.
    ///
    /// The only change asked for here is `STORABLE`, so an answer about another component belongs to
    /// whoever asked - the app through `as_backend`, or an extension this file does not drive.
    pub(super) fn complete_status(
        &mut self,
        request: openxr_sys::AsyncRequestIdFB,
        result: openxr_sys::Result,
        component: openxr_sys::SpaceComponentTypeFB,
        enabled: bool,
    ) {
        let Some(index) = self.slots.iter().position(|slot| {
            matches!(
                slot,
                Some(Slot::Live {
                    saved: Saved::Enabling(id),
                    ..
                }) if *id == request
            )
        }) else {
            return;
        };
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: enabling an anchor component: {result:?}");
            // Not supported is the runtime saying this entity will never be storable; anything else - a
            // timeout, a lost focus - is worth another ask.
            let state = if result == openxr_sys::Result::ERROR_SPACE_COMPONENT_NOT_SUPPORTED_FB {
                Saved::Refused
            } else {
                Saved::Unstored
            };
            self.saved(index, state);
            return;
        }
        if !enabled || component != openxr_sys::SpaceComponentTypeFB::STORABLE {
            // Not this ask: back to unstored, so a later call can ask again rather than wait for an answer
            // that has already come.
            self.saved(index, Saved::Unstored);
            return;
        }
        let Some(Slot::Live { space, .. }) = self.slots.get(index).and_then(Option::as_ref) else {
            return;
        };
        let space = space.as_raw();
        self.save(index, space);
    }

    /// What a save turned out to be.
    pub(super) fn complete_save(
        &mut self,
        request: openxr_sys::AsyncRequestIdFB,
        result: openxr_sys::Result,
    ) {
        let Some(index) = self.slots.iter().position(|slot| {
            matches!(
                slot,
                Some(Slot::Live {
                    saved: Saved::Saving(id),
                    ..
                }) if *id == request
            )
        }) else {
            return;
        };
        if result == openxr_sys::Result::SUCCESS {
            self.saved(index, Saved::Stored);
        } else {
            log::debug!("wxr-openxr: saving an anchor: {result:?}");
            self.saved(index, Saved::Unstored);
        }
    }

    /// What a discovery turned out to be, which is the answer [`OpenXrSession::restore_anchor_impl`] is
    /// waiting on - or the names a listing was asked for, which wait in the table rather than in a slot.
    ///
    /// Results are taken on *completion* rather than on results-available, which the spec allows and which
    /// is the only order without a race: results can arrive between the two calls of the two-call idiom
    /// while a query is still filling, and the runtime answers that with `XR_ERROR_SIZE_INSUFFICIENT` rather
    /// than the space. Once the completion is in, the set is final.
    pub(super) fn complete_discovery(
        &mut self,
        request: openxr_sys::AsyncRequestIdFB,
        result: openxr_sys::Result,
    ) {
        if self.listing == Some(request) {
            return self.complete_listing(request, result);
        }
        let index = self.slots.iter().position(|slot| {
            matches!(
                slot,
                Some(Slot::Restoring { request: id, .. }) if *id == request
            )
        });
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: looking an anchor up: {result:?}");
            if let Some(index) = index {
                self.slots[index] = None;
            }
            return;
        }
        let mut found = self.retrieve(request).into_iter();
        let first = found.next();
        // Anything else that came back is a space this session owns now, and the core has nowhere to put a
        // second one: it is dropped, which is `xrDestroySpace`.
        for (space, _) in found {
            // SAFETY: the runtime returned this handle from our query, so it is a space belonging to this
            // session, and the `Arc` inside keeps the session alive until the drop.
            drop(unsafe { xr::Space::reference_from_raw(self.session.clone(), space) });
        }
        match (index, first) {
            (Some(index), Some((space, _))) => {
                // The name the caller asked for, rather than the UUID the runtime answered with: the query
                // filters on that UUID, so they are the same entity, and answering the bytes the caller
                // wrote down is what makes a round trip through `anchor_id` land on the same name.
                let Some(Slot::Restoring { id, .. }) =
                    self.slots.get(index).and_then(Option::as_ref)
                else {
                    return;
                };
                let id = *id;
                // SAFETY: the runtime returned this handle from our query, so it belongs to this session,
                // and the slot is the only owner from here on.
                let space = unsafe { xr::Space::reference_from_raw(self.session.clone(), space) };
                self.slots[index] = Some(Slot::Live {
                    space,
                    id,
                    // A place the runtime had to have stored to find, so there is nothing left to save.
                    saved: Saved::Stored,
                });
            }
            // A name the runtime does not have empties the handle, which is the core's answer for one.
            (Some(index), None) => self.slots[index] = None,
            (None, Some((space, _))) => {
                // The caller released the anchor while the runtime was looking. The space is still ours, so
                // it is dropped rather than kept with nowhere to put it.
                drop(unsafe { xr::Space::reference_from_raw(self.session.clone(), space) });
            }
            (None, None) => {}
        }
    }

    /// What an erase turned out to be. Nothing needs the space - the name was the whole request - so the
    /// only work is forgetting the request.
    pub(super) fn complete_erase(
        &mut self,
        request: openxr_sys::AsyncRequestIdFB,
        result: openxr_sys::Result,
    ) {
        let Some(persistence) = self.persistence.as_mut() else {
            return;
        };
        let Some(index) = persistence.erases.iter().position(|id| *id == request) else {
            return;
        };
        persistence.erases.swap_remove(index);
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: erasing an anchor's name: {result:?}");
        }
    }

    /// Take the spaces a discovery found, which are this session's from the moment they are retrieved.
    pub(super) fn retrieve(
        &self,
        request: openxr_sys::AsyncRequestIdFB,
    ) -> Vec<(openxr_sys::Space, openxr_sys::UuidEXT)> {
        let Some(persistence) = self.persistence.as_ref() else {
            return Vec::new();
        };
        let mut results = openxr_sys::SpaceDiscoveryResultsMETA {
            ty: openxr_sys::StructureType::SPACE_DISCOVERY_RESULTS_META,
            next: std::ptr::null(),
            result_capacity_input: 0,
            result_count_output: 0,
            results: std::ptr::null_mut(),
        };
        // SAFETY: the session is live, the table is the runtime's own, and `results` outlives both calls.
        // The first call asks for the count and is written for a null array.
        let result = unsafe {
            (persistence.discovery.retrieve_space_discovery_results)(
                self.session.as_raw(),
                request,
                &mut results,
            )
        };
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: counting a discovery's results: {result:?}");
            return Vec::new();
        }
        let count = results.result_count_output as usize;
        if count == 0 {
            return Vec::new();
        }
        let mut buffer: Vec<openxr_sys::SpaceDiscoveryResultMETA> = Vec::with_capacity(count);
        results.result_capacity_input = results.result_count_output;
        results.results = buffer.as_mut_ptr();
        // SAFETY: the capacity is the count the runtime just gave, the array is as large as it asks for, and
        // the completion means the set cannot have grown between the two calls.
        let result = unsafe {
            (persistence.discovery.retrieve_space_discovery_results)(
                self.session.as_raw(),
                request,
                &mut results,
            )
        };
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: reading a discovery's results: {result:?}");
            return Vec::new();
        }
        // SAFETY: the runtime filled `result_count_output` entries of an array with room for at least that
        // many, and every field is plain data.
        unsafe { buffer.set_len(results.result_count_output as usize) };
        buffer
            .into_iter()
            .map(|found| (found.space, found.uuid))
            .collect()
    }

    /// Write what a slot's name is doing, if the slot is still live.
    fn saved(&mut self, index: usize, state: Saved) {
        if let Some(Some(Slot::Live { saved, .. })) = self.slots.get_mut(index) {
            *saved = state;
        }
    }
}

impl OpenXrSession {
    /// The runtime's name for an anchor, which is the UUID the completion carried once the runtime has also
    /// stored the place it names.
    ///
    /// The UUID exists from the moment the anchor does, so answering it immediately would be easy and would
    /// be a lie: nothing has been stored at that point, and a name that finds nothing next run is worse than
    /// no name. So this starts the save the first time it is asked and answers only when the runtime has
    /// confirmed it, which is what the core's `Option` is for.
    pub(super) fn anchor_id_impl(&mut self, anchor: wxr::Anchor) -> Option<wxr::AnchorId> {
        self.anchors.as_mut()?.name(anchor)
    }

    /// Ask the runtime to find a place it stored under `id`, which is `xrDiscoverSpacesMETA` under a UUID
    /// filter.
    ///
    /// The handle comes back now and the space arrives when the discovery completes, which is the same shape
    /// [`OpenXrSession::anchor_impl`] has: what the caller holds is the name of an answer, and `anchor_pose`
    /// is where the answer shows up.
    pub(super) fn restore_anchor_impl(
        &mut self,
        id: wxr::AnchorId,
    ) -> Result<wxr::Anchor, wxr::Error> {
        let Some(anchors) = self.anchors.as_mut() else {
            return Err(unsupported("anchors from an earlier session"));
        };
        let Some(persistence) = anchors.persistence.as_ref() else {
            return Err(unsupported("anchors from an earlier session"));
        };
        let uuid = openxr_sys::UuidEXT {
            data: *id.as_bytes(),
        };
        let filter = openxr_sys::SpaceFilterUuidMETA {
            ty: openxr_sys::StructureType::SPACE_FILTER_UUID_META,
            next: std::ptr::null(),
            uuid_count: 1,
            uuids: &uuid,
        };
        let filters = [&filter as *const openxr_sys::SpaceFilterUuidMETA
            as *const openxr_sys::SpaceFilterBaseHeaderMETA];
        let info = openxr_sys::SpaceDiscoveryInfoMETA {
            ty: openxr_sys::StructureType::SPACE_DISCOVERY_INFO_META,
            next: std::ptr::null(),
            filter_count: 1,
            filters: filters.as_ptr(),
        };
        let mut request: openxr_sys::AsyncRequestIdFB = Default::default();
        // SAFETY: the table is the runtime's own, the session is live, and `uuid`, `filter`, `filters`,
        // `info` and `request` all outlive the call.
        let result = unsafe {
            (persistence.discovery.discover_spaces)(anchors.session.as_raw(), &info, &mut request)
        };
        if result != openxr_sys::Result::SUCCESS {
            return Err(wxr::Error::Unsupported(
                format!("looking an anchor up: the runtime would not: {result:?}").into(),
            ));
        }
        let index = anchors.slots.len() as u32;
        anchors.slots.push(Some(Slot::Restoring { request, id }));
        Ok(wxr::Anchor::new(index))
    }

    /// Erase a name, which is `xrEraseSpacesMETA` with the UUIDs themselves.
    ///
    /// No lookup first: the call takes names rather than spaces, and the spec says a name that was never
    /// saved is a no-op rather than an error - which is exactly the core's contract for this method.
    pub(super) fn delete_persistent_anchor_impl(
        &mut self,
        id: wxr::AnchorId,
    ) -> Result<(), wxr::Error> {
        let Some(anchors) = self.anchors.as_mut() else {
            return Err(unsupported("erasing an anchor's name"));
        };
        let Some(persistence) = anchors.persistence.as_mut() else {
            return Err(unsupported("erasing an anchor's name"));
        };
        let mut uuid = openxr_sys::UuidEXT {
            data: *id.as_bytes(),
        };
        let info = openxr_sys::SpacesEraseInfoMETA {
            ty: openxr_sys::StructureType::SPACES_ERASE_INFO_META,
            next: std::ptr::null(),
            // No spaces, one name: the spec requires at least one of the two and allows either.
            space_count: 0,
            spaces: std::ptr::null_mut(),
            uuid_count: 1,
            uuids: &mut uuid,
        };
        let mut request: openxr_sys::AsyncRequestIdFB = Default::default();
        // SAFETY: the table is the runtime's own, the session is live, and `uuid`, `info` and `request`
        // outlive the call.
        let result = unsafe {
            (persistence.storage.erase_spaces)(anchors.session.as_raw(), &info, &mut request)
        };
        if result != openxr_sys::Result::SUCCESS {
            return Err(wxr::Error::Unsupported(
                format!("erasing an anchor's name: the runtime would not: {result:?}").into(),
            ));
        }
        persistence.erases.push(request);
        Ok(())
    }
}
