//! ARKit's world tracking, and the one question this backend asks of it that the compositor cannot answer:
//! whether the head's place in the room is measured.
//!
//! `CompositorServices` places the eyes every frame and says nothing about how sure it is. The device anchor
//! is ARKit's own answer, and it is read here already because the drawable takes it to reproject against.
//! What was missing is the other half of that answer: `ar_device_anchor_tracking_state_t`, which says
//! whether ARKit is tracking both position and orientation, orientation alone, or neither - the fact the
//! core calls [`wxr::Frame::emulated_position`]. A scene that pins a menu to a wall wants to know when the
//! wall is travelling with the wearer instead, and before this the backend said `false` for every frame of
//! every session, including the ones where ARKit had a direction and no place.
//!
//! The provider can be absent - `is_supported` said no, or the wearer refused the room - and then there is
//! nothing to ask. The answer stays `false` rather than becoming a guess of this backend's own: a session
//! that cannot measure the head has not thereby measured that the head is somewhere else.

use std::ffi::c_void;

use objc2::rc::Retained;
use objc2_ar_kit::{
    ar_data_providers_t, ar_device_anchor_query_status_t, ar_device_anchor_t,
    ar_device_anchor_tracking_state_t, ar_world_tracking_configuration_t,
    ar_world_tracking_provider_t,
};
use wxr::glam::Mat4;

use super::{ArKit, as_data_provider};
use crate::sys;

/// Where the device is at an instant, and whether ARKit measured that place.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Device {
    /// `origin from device`: device space seen from the world's origin, which for a device anchor is where
    /// the head is in the world.
    pub transform: Mat4,
    /// Whether the position is inferred rather than measured - the core's
    /// [`wxr::Frame::emulated_position`], which is what a 3DOF or untracked device anchor reports.
    pub emulated: bool,
}

/// World tracking: the provider and the one anchor its queries refill.
pub(super) struct World {
    pub(super) provider: Retained<ar_world_tracking_provider_t>,
    pub(super) anchor: Retained<ar_device_anchor_t>,
}

impl ArKit {
    /// Start world tracking, or `None` on a device that cannot track a head.
    ///
    /// # Safety
    ///
    /// `providers` must be a live collection.
    pub(super) unsafe fn start_world(providers: &ar_data_providers_t) -> Option<World> {
        // SAFETY: the calls build ARKit objects, and the provider is added to the collection the session
        // retains. The cast is the subtype one documented on `as_data_provider`.
        unsafe {
            if !ar_world_tracking_provider_t::is_supported() {
                return None;
            }
            let configuration = ar_world_tracking_configuration_t::new();
            let provider = ar_world_tracking_provider_t::new(&configuration);
            ar_data_providers_t::add_data_provider(providers, as_data_provider(&provider));
            let anchor = ar_device_anchor_t::new();
            Some(World { provider, anchor })
        }
    }

    /// Whether there is a world to put things in, as opposed to one that follows the wearer.
    pub fn is_world_tracked(&self) -> bool {
        self.world.is_some()
    }

    /// Where the device is at `time`, in seconds from the same epoch the frame's timing is in, and whether
    /// that place was measured.
    ///
    /// The two halves come from one query and are one answer: `None` is tracking that has not come up, which
    /// is a frame to draw head-locked rather than a frame to fail, and a state read separately from a failed
    /// query would be the last successful one.
    pub fn device(&self, time: f64) -> Option<Device> {
        let world = self.world.as_ref()?;
        // SAFETY: the provider and the anchor are both live, and the anchor is ours to fill.
        let status = unsafe {
            ar_world_tracking_provider_t::query_device_anchor_at_timestamp(
                &world.provider,
                time,
                &world.anchor,
            )
        };
        if status != ar_device_anchor_query_status_t::success {
            return None;
        }
        // SAFETY: the anchor is live, the query above filled it, and reading its state does not consume it.
        let state = unsafe { ar_device_anchor_t::tracking_state(&world.anchor) };
        Some(Device {
            transform: Self::transform(Retained::as_ptr(&world.anchor).cast()),
            emulated: emulated(state),
        })
    }

    /// The anchor the last query filled in, which the compositor wants so it can reproject the frame.
    pub fn anchor(&self) -> sys::ArDeviceAnchor {
        match &self.world {
            Some(world) => Retained::as_ptr(&world.anchor) as *mut c_void,
            None => std::ptr::null_mut(),
        }
    }

    /// The transform an anchor is at, as glam's matrix.
    fn transform(anchor: *const c_void) -> Mat4 {
        // SAFETY: the anchor is live, and reading a transform from one does not consume it.
        let transform = unsafe { sys::ar_anchor_get_origin_from_anchor_transform(anchor) };
        Mat4::from_cols_array(&transform.0)
    }
}

/// Whether a tracking state means the device's position is inferred rather than measured.
///
/// `orientation_tracked` is the 3DOF case - a direction with a place underneath it that ARKit is not
/// measuring - and `untracked` is a place ARKit is not claiming at all. Both are what the core means by an
/// emulated position, because neither is an answer to where the head is; only `tracked` is.
fn emulated(state: ar_device_anchor_tracking_state_t) -> bool {
    state != ar_device_anchor_tracking_state_t::tracked
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only the state that has both halves is a measured place, and the other two are not.
    #[test]
    fn only_a_fully_tracked_device_has_a_measured_place() {
        assert!(!emulated(ar_device_anchor_tracking_state_t::tracked));
        // A direction with no measured place, and no answer at all, are both a place to be guessed at.
        assert!(emulated(
            ar_device_anchor_tracking_state_t::orientation_tracked
        ));
        assert!(emulated(ar_device_anchor_tracking_state_t::untracked));
    }
}
