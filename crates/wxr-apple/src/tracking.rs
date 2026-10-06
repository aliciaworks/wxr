//! ARKit's world tracking, which is what makes a pose mean somewhere.
//!
//! Without it, every pose a compositor reports is relative to the device and the scene is head-locked: it
//! follows the wearer instead of staying where it was put. With it there is an origin fixed where the
//! session began, and that origin is exactly the core's [`wxr::SpaceKind::Local`] - which is why this is the
//! module that turns that space from a refusal into an answer.
//!
//! What is *not* here is a floor. ARKit's origin is not a plane and nothing in these calls says where the
//! floor is, so [`wxr::SpaceKind::LocalFloor`] stays refused - a scene put at eye height is worse than a
//! scene told no.
//!
//! Tracking is allowed to fail. A refused provider - usually a missing `NSWorldSensingUsageDescription` -
//! leaves the scene head-locked, which still draws, so everything here comes back as an `Option` rather than
//! as an error to stop a frame over.

use std::ffi::c_void;

use wxr::glam::Mat4;

use crate::sys;

/// An ARKit object, released on drop.
///
/// Every `ar_*_create` is a `+1` and `ar_release` is what gives it back, the same rule as Core Foundation.
/// Without this the session, the provider and the anchor would each leak once per session, which is small
/// and is still not a thing to leave lying in a backend that is supposed to be an example.
struct Object(*mut c_void);

impl Object {
    fn new(pointer: *mut c_void) -> Option<Self> {
        (!pointer.is_null()).then_some(Self(pointer))
    }

    fn pointer(&self) -> *mut c_void {
        self.0
    }
}

impl Drop for Object {
    fn drop(&mut self) {
        // SAFETY: the pointer is one of ours, made by an `ar_*_create` and not released yet.
        unsafe { sys::ar_release(self.0) };
    }
}

/// Where the headset is, in the world ARKit tracks.
pub struct WorldTracking {
    /// Kept for its lifetime: stopping the session would stop the provider with it.
    _session: Object,
    provider: Object,
    /// One anchor, refilled by every query - ARKit writes into it rather than handing back a new one, which
    /// is also why the compositor can be given it afterwards.
    anchor: Object,
}

impl WorldTracking {
    /// Start a session with a world tracking provider, or `None` if ARKit will not give one.
    pub fn new() -> Option<Self> {
        // SAFETY: every call below creates an object that is checked for null and then owned by this struct
        // or retained by the session, and each is released exactly once by `Object`'s drop.
        unsafe {
            let configuration = Object::new(sys::ar_world_tracking_configuration_create())?;
            let provider = Object::new(sys::ar_world_tracking_provider_create(
                configuration.pointer(),
            ))?;

            let providers = Object::new(sys::ar_data_providers_create())?;
            sys::ar_data_providers_add_data_provider(providers.pointer(), provider.pointer());

            let session = Object::new(sys::ar_session_create())?;
            // The session retains the collection, so it is dropped here with the session still holding it -
            // which is why it is a local and not a field.
            sys::ar_session_run(session.pointer(), providers.pointer());

            let anchor = Object::new(sys::ar_device_anchor_create())?;
            Some(Self {
                _session: session,
                provider,
                anchor,
            })
        }
    }

    /// Where the device is at `time`, in seconds from the same epoch the frame's timing is in.
    ///
    /// `None` while tracking is coming up, which is a frame to draw head-locked rather than a frame to fail.
    /// The result is `origin from device`: device space seen from the world's origin, which for a
    /// device anchor is where the head is in the world.
    pub fn device(&self, time: f64) -> Option<Mat4> {
        // SAFETY: the provider and the anchor are both live, and the anchor is ours to fill.
        let status = unsafe {
            sys::ar_world_tracking_provider_query_device_anchor_at_timestamp(
                self.provider.pointer(),
                time,
                self.anchor.pointer(),
            )
        };
        if status != sys::QUERY_SUCCESS {
            return None;
        }
        // SAFETY: the query just filled the anchor in, and reading a transform from an anchor does not
        // consume it.
        let transform =
            unsafe { sys::ar_anchor_get_origin_from_anchor_transform(self.anchor.pointer()) };
        Some(Mat4::from_cols_array(&transform.0))
    }

    /// The anchor the last query filled in, which the compositor wants so it can reproject the frame.
    pub fn anchor(&self) -> sys::ArDeviceAnchor {
        self.anchor.pointer()
    }
}
