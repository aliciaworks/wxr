//! ARKit's session, and the two providers this leg asks it for.
//!
//! One session, because a session is what holds the authorization and the providers: world tracking for
//! where the head is, and hand tracking for where the hands are. The declarations come from the generated
//! `objc2-ar-kit` crate - ARKit read from the SDK that has its visionOS C module - and only the two
//! functions whose signature *carries* a `simd` type stay hand-written in [`crate::sys`], because
//! `objc2`'s translator cannot yet express `simd` in a function. Everything else here is ownership:
//! `Retained` is the `+1` every `create` returns and the `ar_release` that used to be written out by hand.
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
//! **Surfaces are asked about rather than assumed.** The plane provider is on the C surface - [`planes_are_supported`]
//! asks it - but this leg draws no surface yet, so [`wxr::Session::planes`] is the core's default here: a
//! session with no surfaces. [`wxr::Session::anchor`] is the same story; the world-tracking provider this
//! crate uses *queries* a device anchor, and whether the C surface can *add* one is a thing to read before
//! it is written.
//!
//! **Foveation is the one thing this platform has that the core cannot reach, and the reason is `wgpu`.** The
//! compositor does foveation -
//! `cp_layer_renderer_capabilities_supports_foveation` says whether, `cp_layer_renderer_configuration_set_foveation_enabled`
//! turns it on - but it is a *boolean chosen when the layer is configured*, and the drawing it saves comes from
//! the `MTLRasterizationRateMap` a drawable hands over (`cp_drawable_get_rasterization_rate_map`), which the
//! app has to attach to the render passes it draws into. `wgpu` has no way to attach one, so a pass drawn by
//! this workspace's renderer cannot be foveated however the layer is configured - and [`wxr::Session::set_foveation`]
//! is the core's default here: the choice is the app's and the drawing is the renderer's, not this backend's.
//!
//! Tracking is allowed to fail. A refused provider - usually a missing `NSWorldSensingUsageDescription` -
//! leaves the scene head-locked and the hands absent, both of which still draw, so everything here comes
//! back as an `Option` rather than as an error to stop a frame over.

use std::ffi::c_void;

use objc2::rc::Retained;
use objc2_ar_kit::{
    ar_data_provider_t, ar_data_providers_t, ar_device_anchor_query_status_t, ar_device_anchor_t,
    ar_hand_anchor_t, ar_hand_tracking_configuration_t, ar_hand_tracking_provider_t,
    ar_plane_alignment_t, ar_plane_anchor_t, ar_plane_anchors_t,
    ar_plane_detection_configuration_t, ar_plane_detection_provider_t, ar_plane_extent_t,
    ar_plane_geometry_t, ar_session_t, ar_trackable_anchor_t, ar_world_tracking_configuration_t,
    ar_world_tracking_provider_t,
};
use wxr::glam::Mat4;

use crate::sys;

/// Reinterpret a concrete provider as the `ar_data_provider_t` a collection takes.
///
/// `objc2`'s generator gives every opaque ARKit handle its own type and does not model which is a subtype of
/// which, but the C header declares the concrete providers as `ar_data_provider_t` subclasses and hands over
/// the same pointer. This is that declaration, made explicit rather than assumed.
///
/// # Safety
///
/// `provider` must be one of ARKit's provider objects, all of which begin with an `ar_data_provider_t`.
unsafe fn as_data_provider<T>(provider: &T) -> &ar_data_provider_t {
    // SAFETY: the caller guarantees the object begins with an `ar_data_provider_t`.
    unsafe { &*std::ptr::from_ref(provider).cast::<ar_data_provider_t>() }
}

/// Reinterpret an anchor as the `ar_trackable_anchor_t` the tracked query takes, by the same rule.
///
/// # Safety
///
/// `anchor` must be an ARKit anchor that conforms to `OS_ar_trackable_anchor`.
unsafe fn as_trackable_anchor<T>(anchor: &T) -> &ar_trackable_anchor_t {
    // SAFETY: the caller guarantees the object begins with an `ar_trackable_anchor_t`.
    unsafe { &*std::ptr::from_ref(anchor).cast::<ar_trackable_anchor_t>() }
}

/// World tracking: the provider and the one anchor its queries refill.
struct World {
    provider: Retained<ar_world_tracking_provider_t>,
    anchor: Retained<ar_device_anchor_t>,
}

/// Hand tracking: the provider and an anchor for each hand.
struct Hands {
    provider: Retained<ar_hand_tracking_provider_t>,
    left: Retained<ar_hand_anchor_t>,
    right: Retained<ar_hand_anchor_t>,
}

/// Plane detection: the provider whose anchors are the surfaces, read fresh each time they are asked for.
struct Planes {
    provider: Retained<ar_plane_detection_provider_t>,
}

/// ARKit, with whichever providers came up.
pub struct ArKit {
    /// Kept for its lifetime: stopping the session would stop the providers with it.
    _session: Retained<ar_session_t>,
    /// The collection the session was run with. The session retains it; this holds it so that the providers
    /// cannot be released before the session has been told.
    _providers: Retained<ar_data_providers_t>,
    world: Option<World>,
    hands: Option<Hands>,
    planes: Option<Planes>,
}

impl ArKit {
    /// Start a session with the providers this device supports, or `None` if it gave none of them.
    pub fn new() -> Option<Self> {
        // SAFETY: each `new` returns a `Retained` - ARKit's creators are non-null by the header - and the
        // providers are handed to the session, which retains them.
        unsafe {
            let session = ar_session_t::new();
            let providers = ar_data_providers_t::new();

            let world = Self::start_world(&providers);
            let hands = Self::start_hands(&providers);
            let planes = Self::start_planes(&providers);
            if world.is_none() && hands.is_none() {
                return None;
            }

            // The session retains the collection, so this is the last this code has to think about it.
            ar_session_t::run(&session, &providers);
            Some(Self {
                _session: session,
                _providers: providers,
                world,
                hands,
                planes,
            })
        }
    }

    /// # Safety
    ///
    /// `providers` must be a live collection.
    unsafe fn start_world(providers: &ar_data_providers_t) -> Option<World> {
        // SAFETY: the calls build ARKit objects, and the provider is added to the collection the session
        // retains. The casts are the subtype ones documented on the helpers above.
        unsafe {
            let configuration = ar_world_tracking_configuration_t::new();
            let provider = ar_world_tracking_provider_t::new(&configuration);
            ar_data_providers_t::add_data_provider(providers, as_data_provider(&provider));
            let anchor = ar_device_anchor_t::new();
            Some(World { provider, anchor })
        }
    }

    /// # Safety
    ///
    /// `providers` must be a live collection.
    unsafe fn start_hands(providers: &ar_data_providers_t) -> Option<Hands> {
        // SAFETY: as above. Support is asked first, because a provider on a device that has none is a
        // provider whose every query fails.
        unsafe {
            if !ar_hand_tracking_provider_t::is_supported() {
                return None;
            }
            let configuration = ar_hand_tracking_configuration_t::new();
            let provider = ar_hand_tracking_provider_t::new(&configuration);
            ar_data_providers_t::add_data_provider(providers, as_data_provider(&provider));
            let left = ar_hand_anchor_t::new();
            let right = ar_hand_anchor_t::new();
            Some(Hands {
                provider,
                left,
                right,
            })
        }
    }

    /// # Safety
    ///
    /// `providers` must be a live collection.
    unsafe fn start_planes(providers: &ar_data_providers_t) -> Option<Planes> {
        // SAFETY: as above. Support is asked first, because a provider on a device that has none is a
        // provider whose every query fails.
        unsafe {
            if !ar_plane_detection_provider_t::is_supported() {
                return None;
            }
            let configuration = ar_plane_detection_configuration_t::new();
            let provider = ar_plane_detection_provider_t::new(&configuration);
            ar_data_providers_t::add_data_provider(providers, as_data_provider(&provider));
            Some(Planes { provider })
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
            ar_world_tracking_provider_t::query_device_anchor_at_timestamp(
                &world.provider,
                time,
                &world.anchor,
            )
        };
        if status != ar_device_anchor_query_status_t::success {
            return None;
        }
        Some(Self::transform(Retained::as_ptr(&world.anchor).cast()))
    }

    /// The anchor the last query filled in, which the compositor wants so it can reproject the frame.
    pub fn anchor(&self) -> sys::ArDeviceAnchor {
        match &self.world {
            Some(world) => Retained::as_ptr(&world.anchor) as *mut c_void,
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
            ar_hand_tracking_provider_t::latest_anchors(&hands.provider, &hands.left, &hands.right)
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
            // SAFETY: both casts are the subtype ones above; the anchor is live and the transform is the
            // hand's place in the world.
            let transform = unsafe {
                sys::ar_hand_anchor_get_origin_from_anchor_transform(
                    Retained::as_ptr(anchor).cast(),
                )
            };
            let tracked = unsafe { ar_trackable_anchor_t::is_tracked(as_trackable_anchor(anchor)) };
            (handedness, Mat4::from_cols_array(&transform.0), tracked)
        })
        .collect()
    }

    /// The transform an anchor is at, as glam's matrix.
    fn transform(anchor: *const c_void) -> Mat4 {
        // SAFETY: the anchor is live, and reading a transform from one does not consume it.
        let transform = unsafe { sys::ar_anchor_get_origin_from_anchor_transform(anchor) };
        Mat4::from_cols_array(&transform.0)
    }

    /// Whether a plane provider came up, which is what `features` asks.
    pub fn has_planes(&self) -> bool {
        self.planes.is_some()
    }

    /// The surfaces ARKit is tracking, which is where `planes` gets them.
    ///
    /// A copy and not a cache: the anchors are read fresh each call, because a plane is re-tracked as ARKit
    /// learns more about the room and a stale copy is the one thing the core's `Plane` is meant not to be.
    pub fn planes(&self) -> Vec<wxr::Plane> {
        let Some(planes) = &self.planes else {
            return Vec::new();
        };
        // SAFETY: the provider is live, and the enumerator only reads the anchors it is handed.
        unsafe {
            let anchors = ar_plane_detection_provider_t::all_plane_anchors(&planes.provider);
            let mut out = Vec::new();
            let context = std::ptr::from_mut(&mut out).cast::<c_void>();
            ar_plane_anchors_t::enumerate_anchors_f(&anchors, context, collect_plane);
            out
        }
    }
}

/// One plane anchor, as the core's [`wxr::Plane`].
///
/// The pose is the anchor's transform, which ARKit builds with the surface's normal on +Y and its extent
/// along X and Z - the same space WebXR's plane space is, and the one the core says a plane's pose is in.
///
/// # Safety
///
/// `anchor` must be a live plane anchor.
unsafe fn plane_of(anchor: &ar_plane_anchor_t) -> wxr::Plane {
    // SAFETY: the caller guarantees the anchor is live; the transform is where ARKit put it, and the
    // geometry is the extent it reports for it.
    unsafe {
        let transform = sys::ar_anchor_get_origin_from_anchor_transform(
            std::ptr::from_ref(anchor).cast::<c_void>(),
        );
        let (_, orientation, position) =
            Mat4::from_cols_array(&transform.0).to_scale_rotation_translation();
        let geometry = ar_plane_anchor_t::geometry(anchor);
        let extent = ar_plane_geometry_t::plane_extent(&geometry);
        let mut identifier = [0u8; 16];
        ar_plane_anchor_t::identifier(anchor, &mut identifier);
        let alignment = ar_plane_anchor_t::alignment(anchor);
        wxr::Plane {
            // The anchor's UUID folded to the name the core carries: the same 32 bits for as long as the
            // anchor lives, which is what a stable id has to be.
            id: u32::from_le_bytes([identifier[0], identifier[1], identifier[2], identifier[3]]),
            pose: wxr::Pose {
                position,
                orientation,
            },
            extent: wxr::glam::Vec2::new(
                ar_plane_extent_t::width(&extent),
                ar_plane_extent_t::height(&extent),
            ),
            orientation: match alignment {
                a if a == ar_plane_alignment_t::horizontal => wxr::PlaneOrientation::Horizontal,
                a if a == ar_plane_alignment_t::vertical => wxr::PlaneOrientation::Vertical,
                _ => wxr::PlaneOrientation::Unknown,
            },
        }
    }
}

/// The enumerator `planes` hands to ARKit: one plane into the `Vec` the context points at.
///
/// # Safety
///
/// `context` must be the `*mut Vec<wxr::Plane>` the caller passed, and `anchor` a live plane anchor.
unsafe extern "C-unwind" fn collect_plane(
    context: *mut c_void,
    anchor: &ar_plane_anchor_t,
) -> bool {
    // SAFETY: the caller guarantees both.
    unsafe {
        (*context.cast::<Vec<wxr::Plane>>()).push(plane_of(anchor));
    }
    true
}

/// Whether the platform reports that plane detection is supported.
pub fn planes_are_supported() -> bool {
    unsafe { objc2_ar_kit::ar_plane_detection_provider_t::is_supported() }
}
