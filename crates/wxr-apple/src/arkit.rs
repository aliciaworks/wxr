//! ARKit's session, and the two providers this leg asks it for.
//!
//! One session, because a session is what holds the authorization and the providers: world tracking for
//! where the head is, and hand tracking for where the hands are. All of it is C - see [`crate::sys`] - so
//! there is nothing here but lifetimes and two queries.
//!
//! **World tracking** is what makes a pose mean somewhere. Without it every pose a compositor reports is
//! relative to the device and the scene follows the wearer instead of staying where it was put; with it
//! there is an origin fixed where the session began, which is exactly the core's [`wxr::SpaceKind::Local`].
//! What is *not* here is a floor: ARKit's origin is not a plane, so `LocalFloor` stays refused rather than a
//! scene being put at eye height.
//!
//! **Hand tracking** is the input on this platform, and it is worth being exact about how much of it the C
//! API gives. It gives a provider, an anchor per hand, and each anchor's place in the world - and that is
//! all: the skeleton is not in the C API, only the enumeration of joint *names* is, so there are no joint
//! transforms to read, no pinch to detect and no ray to derive from a fingertip. A hand here is therefore a
//! place and an orientation, and its `aim` is its `grip` - see `session`'s `inputs`. A game that needs a
//! pinch wants the Swift `HandAnchor.Skeleton`, which means the app, which is where gestures belong anyway.
//!
//! Tracking is allowed to fail. A refused provider - usually a missing `NSWorldSensingUsageDescription` -
//! leaves the scene head-locked and the hands absent, both of which still draw, so everything here comes
//! back as an `Option` rather than as an error to stop a frame over.

use std::ffi::c_void;

use wxr::glam::Mat4;

use crate::sys;

/// An ARKit object, released on drop.
///
/// Every `ar_*_create` is a `+1` and `ar_release` is what gives it back, the same rule as Core Foundation.
/// Without this the session, the providers and the anchors would each leak once per session, which is small
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

/// World tracking: the provider and the one anchor its queries refill.
struct World {
    provider: Object,
    anchor: Object,
}

/// Hand tracking: the provider and an anchor for each hand.
struct Hands {
    provider: Object,
    left: Object,
    right: Object,
}

/// ARKit, with whichever providers came up.
pub struct ArKit {
    /// Kept for its lifetime: stopping the session would stop the providers with it.
    _session: Object,
    /// The collection the session was run with. The session retains it; this holds it so that the providers
    /// cannot be released before the session has been told.
    _providers: Object,
    world: Option<World>,
    hands: Option<Hands>,
}

impl ArKit {
    /// Start a session with the providers this device supports, or `None` if it gave none of them.
    pub fn new() -> Option<Self> {
        // SAFETY: every call below creates an object that is checked for null and then owned by this struct
        // or retained by the session, and each is released exactly once by `Object`'s drop.
        unsafe {
            let session = Object::new(sys::ar_session_create())?;
            let providers = Object::new(sys::ar_data_providers_create())?;

            let world = Self::start_world(&providers);
            let hands = Self::start_hands(&providers);
            if world.is_none() && hands.is_none() {
                return None;
            }

            // The session retains the collection, so this is the last this code has to think about it.
            sys::ar_session_run(session.pointer(), providers.pointer());
            Some(Self {
                _session: session,
                _providers: providers,
                world,
                hands,
            })
        }
    }

    fn start_world(providers: &Object) -> Option<World> {
        // SAFETY: each call makes an object that is checked for null, and the provider is added to the
        // collection the session will retain.
        unsafe {
            let configuration = Object::new(sys::ar_world_tracking_configuration_create())?;
            let provider = Object::new(sys::ar_world_tracking_provider_create(
                configuration.pointer(),
            ))?;
            sys::ar_data_providers_add_data_provider(providers.pointer(), provider.pointer());
            let anchor = Object::new(sys::ar_device_anchor_create())?;
            Some(World { provider, anchor })
        }
    }

    fn start_hands(providers: &Object) -> Option<Hands> {
        // SAFETY: as above. Support is asked first, because a provider on a device that has none is a
        // provider whose every query fails.
        unsafe {
            if !sys::ar_hand_tracking_provider_is_supported() {
                return None;
            }
            let configuration = Object::new(sys::ar_hand_tracking_configuration_create())?;
            let provider = Object::new(sys::ar_hand_tracking_provider_create(
                configuration.pointer(),
            ))?;
            sys::ar_data_providers_add_data_provider(providers.pointer(), provider.pointer());
            let left = Object::new(sys::ar_hand_anchor_create())?;
            let right = Object::new(sys::ar_hand_anchor_create())?;
            Some(Hands {
                provider,
                left,
                right,
            })
        }
    }

    /// Whether there is a world to put things in, as opposed to one that follows the wearer.
    pub fn is_world_tracked(&self) -> bool {
        self.world.is_some()
    }

    /// Where the device is at `time`, in seconds from the same epoch the frame's timing is in.
    ///
    /// `None` while tracking is coming up, which is a frame to draw head-locked rather than a frame to fail.
    /// The result is `origin from device`: device space seen from the world's origin, which for a device
    /// anchor is where the head is in the world.
    pub fn device(&self, time: f64) -> Option<Mat4> {
        let world = self.world.as_ref()?;
        // SAFETY: the provider and the anchor are both live, and the anchor is ours to fill.
        let status = unsafe {
            sys::ar_world_tracking_provider_query_device_anchor_at_timestamp(
                world.provider.pointer(),
                time,
                world.anchor.pointer(),
            )
        };
        if status != sys::QUERY_SUCCESS {
            return None;
        }
        Some(Self::transform(world.anchor.pointer()))
    }

    /// The anchor the last query filled in, which the compositor wants so it can reproject the frame.
    pub fn anchor(&self) -> sys::ArDeviceAnchor {
        match &self.world {
            Some(world) => world.anchor.pointer(),
            None => std::ptr::null_mut(),
        }
    }

    /// Where the hands are, as `(handedness, place in the world, tracked)`.
    ///
    /// Empty when there is no hand provider, when it has nothing yet, or when the query failed - all three
    /// of which are a frame with no hands rather than a frame to stop over. Handedness comes from which
    /// anchor it is, because the call hands over a left one and a right one and says so in its arguments.
    pub fn hands(&self) -> Vec<(wxr::Handedness, Mat4, bool)> {
        let Some(hands) = &self.hands else {
            return Vec::new();
        };
        // SAFETY: the provider and both anchors are live, and the anchors are ours to fill.
        let latest = unsafe {
            sys::ar_hand_tracking_provider_get_latest_anchors(
                hands.provider.pointer(),
                hands.left.pointer(),
                hands.right.pointer(),
            )
        };
        if !latest {
            return Vec::new();
        }
        [
            (wxr::Handedness::Left, &hands.left),
            (wxr::Handedness::Right, &hands.right),
        ]
        .into_iter()
        .map(|(handedness, anchor)| {
            // SAFETY: the anchor is live, and both reads are of properties ARKit filled in.
            let transform = unsafe {
                sys::ar_hand_anchor_get_origin_from_anchor_transform_with_correction(
                    anchor.pointer(),
                    sys::TRANSFORM_NONE,
                )
            };
            let tracked = unsafe { sys::ar_trackable_anchor_is_tracked(anchor.pointer()) };
            (handedness, Mat4::from_cols_array(&transform.0), tracked)
        })
        .collect()
    }

    /// The transform an anchor is at, as glam's matrix.
    fn transform(anchor: sys::ArAnchor) -> Mat4 {
        // SAFETY: the anchor is live, and reading a transform from one does not consume it.
        let transform = unsafe { sys::ar_anchor_get_origin_from_anchor_transform(anchor) };
        Mat4::from_cols_array(&transform.0)
    }
}
