//! World anchors: the places ARKit keeps fixed, and the callbacks that add and remove them.
//!
//! Separate from `arkit`'s provider setup for the reason `plane` and `mesh` are: this is one answer about one
//! provider, with its own asynchronous completion, and the module that starts every provider was already the
//! longest of them.

use std::ffi::c_void;

use objc2::rc::Retained;
use objc2_ar_kit::{ar_error_t, ar_world_anchor_t, ar_world_tracking_provider_t};
use wxr::glam::Mat4;

use super::ArKit;
use crate::sys;

impl ArKit {
    /// Whether world anchors can be made, which is the world-tracking provider being up.
    pub fn has_anchors(&self) -> bool {
        self.world.is_some()
    }

    /// A world anchor at `origin_from_anchor`, a transform in the session's own origin.
    ///
    /// ARKit adds an anchor to the world-tracking provider *asynchronously* - the completion says whether
    /// it took - so the handle comes back now and the pose is read later, which is the shape the core asks
    /// for anyway: an anchor is a name to ask by, not a pose to cache.
    ///
    /// `None` when there is no world-tracking provider, because an anchor without one is a place nobody
    /// is keeping.
    pub fn add_anchor(&mut self, origin_from_anchor: Mat4) -> Option<wxr::Anchor> {
        let world = self.world.as_ref()?;
        // SAFETY: the transform is ours, and a create returns a `+1` object that the `Retained` below
        // takes ownership of.
        let raw = unsafe {
            sys::ar_world_anchor_create_with_origin_from_anchor_transform(sys::Float4x4(
                origin_from_anchor.to_cols_array(),
            ))
        };
        // SAFETY: a `+1` object from ARKit, which is exactly what `Retained` owns.
        let world_anchor = unsafe { Retained::from_raw(raw.cast::<ar_world_anchor_t>()) }?;
        // SAFETY: the provider and the anchor are both live, and the completion is a no-op because the
        // anchor is already ours.
        unsafe {
            ar_world_tracking_provider_t::add_anchor_f(
                &world.provider,
                &world_anchor,
                std::ptr::null_mut(),
                anchor_added,
            );
        }
        let name = self.next_anchor;
        self.next_anchor += 1;
        self.anchors.insert(name, world_anchor);
        Some(wxr::Anchor::new(name))
    }

    /// Where an anchor is now, in the session's own origin.
    ///
    /// `None` until the provider has tracked it and while it has lost it, which is what the core's
    /// `anchor_pose` means by a place that is not there. An anchor that was never added, or whose add failed,
    /// is never tracked and so never answers with its creation transform.
    pub fn anchor_pose(&self, anchor: wxr::Anchor) -> Option<Mat4> {
        let world_anchor = self.anchors.get(&anchor.id())?;
        // SAFETY: the anchor is live, and asking whether it is tracked does not consume it.
        if !unsafe { ar_world_anchor_t::is_tracked(world_anchor) } {
            return None;
        }
        // SAFETY: the anchor is live, and reading a transform from one does not consume it.
        let transform = unsafe {
            sys::ar_world_anchor_get_origin_from_anchor_transform(
                std::ptr::from_ref(world_anchor).cast::<c_void>(),
            )
        };
        Some(Mat4::from_cols_array(&transform.0))
    }

    /// Gives an anchor back, which drops it from the runtime as well.
    ///
    /// Dropping the handle is not enough: a world anchor the provider added is persisted across device
    /// restarts until it is explicitly removed, so a released anchor that was only dropped from the map would
    /// stay in the room - and on disk - for every session after this one.
    pub fn release_anchor(&mut self, anchor: wxr::Anchor) {
        let Some(world) = &self.world else {
            return;
        };
        let Some(world_anchor) = self.anchors.remove(&anchor.id()) else {
            return;
        };
        // SAFETY: the provider and the anchor are both live. The completion is a no-op: the anchor has been
        // given back, and the provider holds it until the removal finishes.
        unsafe {
            ar_world_tracking_provider_t::remove_anchor_f(
                &world.provider,
                &world_anchor,
                std::ptr::null_mut(),
                anchor_removed,
            );
        }
    }
}

/// The completion `anchor` hands ARKit: the add is asynchronous, and the anchor is already in the map, so
/// there is nothing here but to not be surprised by the answer.
///
/// # Safety
///
/// Called by ARKit with the context the caller passed.
unsafe extern "C-unwind" fn anchor_added(
    _context: *mut c_void,
    _anchor: &ar_world_anchor_t,
    _success: bool,
    _error: Option<&ar_error_t>,
) {
}

/// The completion for a released anchor, which has nothing left to do: the handle is out of the map and the
/// provider owns the object until it has finished removing it.
///
/// # Safety
///
/// Called by ARKit with the context the caller passed.
unsafe extern "C-unwind" fn anchor_removed(
    _context: *mut c_void,
    _anchor: &ar_world_anchor_t,
    _success: bool,
    _error: Option<&ar_error_t>,
) {
}
