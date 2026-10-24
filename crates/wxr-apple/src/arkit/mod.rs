//! ARKit's session, and the providers this leg asks it for.
//!
//! One session, because a session is what holds the authorization and the providers: world tracking for
//! where the head is, hand tracking for where the hands are, and the two that describe the room - planes,
//! which are the surfaces ARKit recognised, and scene reconstruction, which is the triangles. The
//! declarations come from the generated `objc2-ar-kit` crate - ARKit read from the SDK that has its visionOS
//! C module - and only the two functions whose signature *carries* a `simd` type stay hand-written in
//! [`crate::sys`], because `objc2`'s translator cannot yet express `simd` in a function. Everything else here
//! is ownership: `Retained` is the `+1` every `create` returns and the `ar_release` that used to be written
//! out by hand.
//!
//! **World tracking** is what makes a pose mean somewhere. Without it every pose a compositor reports is
//! relative to the device and the scene follows the wearer instead of staying where it was put; with it
//! there is an origin fixed where the session began, which is exactly the core's [`wxr::SpaceKind::Local`].
//! What is *not* here is a floor: ARKit's origin is not a plane, so `LocalFloor` stays refused rather than a
//! scene being put at eye height.
//!
//! **Hand tracking** is the input on this platform, and it is a skeleton rather than a palm - which is worth
//! saying plainly, because this backend's own documentation once said the opposite. The C API does give
//! joint transforms: `ar_hand_anchor_get_hand_skeleton` hands over a skeleton, `ar_hand_skeleton_get_joint_named`
//! hands back a joint of it, and that joint's `ar_skeleton_joint_get_anchor_from_joint_transform` is where it
//! is. What made it look absent is that the transform returns a `simd_float4x4` by value, which `objc2`'s
//! translator cannot express, so the generated crate binds the joint's index and parent and leaves the
//! transform undeclared. The module `skeleton` is where that transform is read, through [`crate::sys`]'s
//! shim.
//!
//! A hand is therefore a place, an orientation and twenty-five joints - see [`ArKit::hand`]. The one
//! thing still absent is a *pinch*: ARKit's C surface reports joint poses and not the discrete pinch state
//! its Swift API has, so a game that wants one derives it from the thumb tip and index tip, which is what
//! those joints are there for.
//!
//! **The room comes in two resolutions, and both are here.** The plane provider is the surfaces ARKit
//! recognised - [`ArKit::planes`], with [`planes_are_supported`] as the question of whether the device can -
//! and scene reconstruction is the same provider shape one level down: [`ArKit::meshes`] is the triangles
//! themselves, each with what ARKit says it is. Two providers on the platform and two bits in the core, so a
//! session can have either without the other. [`wxr::Session::anchor`] is the world rather than the room: a
//! world anchor is made through the world-tracking provider, which is what [`ArKit::add_anchor`] does.
//!
//! **The room's light is a third description of it, and the only one here that is not a shape.** ARKit's
//! environment probe photographs the room as a cubemap and pushes it over on a queue of its own; the core
//! asks for nine spherical harmonics and a brightest direction once a frame. [`ArKit::light`] is that
//! mismatch and everything under it - an adapter from push to pull, a projection from a cubemap to
//! order-two harmonics, and a conversion out of P3 - with the arithmetic kept apart from the API on
//! purpose, because none of it can be checked without deciding first which half is which.
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

use std::collections::HashMap;
use std::ffi::c_void;

use objc2::rc::Retained;
use objc2_ar_kit::{
    ar_data_provider_t, ar_data_providers_t, ar_device_anchor_query_status_t, ar_device_anchor_t,
    ar_hand_anchor_t, ar_hand_tracking_configuration_t, ar_hand_tracking_provider_t,
    ar_mesh_anchors_t, ar_plane_anchors_t, ar_plane_detection_configuration_t,
    ar_plane_detection_provider_t, ar_scene_reconstruction_configuration_t,
    ar_scene_reconstruction_provider_t, ar_session_t, ar_trackable_anchor_t, ar_world_anchor_t,
    ar_world_tracking_configuration_t, ar_world_tracking_provider_t,
};
use wxr::glam::Mat4;

use crate::sys;

mod anchor;
mod light;
mod mesh;
mod plane;
mod skeleton;

use light::Lights;
use mesh::collect_mesh;
use plane::collect_plane;

/// Reinterpret a concrete provider as the `ar_data_provider_t` a collection takes.
///
/// `objc2`'s generator gives every opaque ARKit handle its own type and does not model which is a subtype of
/// which, but the C header declares the concrete providers as `ar_data_provider_t` subclasses and hands over
/// the same pointer. This is that declaration, made explicit rather than assumed.
///
/// # Safety
///
/// `provider` must be one of ARKit's provider objects, all of which begin with an `ar_data_provider_t`.
unsafe fn as_data_provider<T>(provider: &Retained<T>) -> &ar_data_provider_t {
    // The pointer to hand over is the object the `Retained` holds, not the address of the `Retained`
    // variable that holds it - ARKit retains the object, so a handle's address is not its object. The
    // deref goes through `Retained` to the object, and the address of that is the object pointer.
    // SAFETY: the caller guarantees the object begins with an `ar_data_provider_t`.
    unsafe { &*std::ptr::from_ref(&**provider).cast::<ar_data_provider_t>() }
}

/// Reinterpret an anchor as the `ar_trackable_anchor_t` the tracked query takes, by the same rule.
///
/// # Safety
///
/// `anchor` must be an ARKit anchor that conforms to `OS_ar_trackable_anchor`.
unsafe fn as_trackable_anchor<T>(anchor: &Retained<T>) -> &ar_trackable_anchor_t {
    // As above: the object the `Retained` holds, not the address of the handle.
    // SAFETY: the caller guarantees the object begins with an `ar_trackable_anchor_t`.
    unsafe { as_trackable(&**anchor) }
}

/// The same for an anchor the runtime hands over as a borrow, which is how the enumerators carry a plane or a
/// mesh.
///
/// # Safety
///
/// `anchor` must be an ARKit anchor that conforms to `OS_ar_trackable_anchor`.
pub(super) unsafe fn as_trackable<T>(anchor: &T) -> &ar_trackable_anchor_t {
    // SAFETY: the caller guarantees the object begins with an `ar_trackable_anchor_t`.
    unsafe { &*std::ptr::from_ref(anchor).cast::<ar_trackable_anchor_t>() }
}

/// Whether an anchor the runtime is enumerating is tracked this frame.
///
/// # Safety
///
/// `anchor` must be a live ARKit anchor that conforms to `OS_ar_trackable_anchor`.
pub(super) unsafe fn is_tracked<T>(anchor: &T) -> bool {
    // SAFETY: the caller guarantees both the live anchor and its conformance.
    unsafe { ar_trackable_anchor_t::is_tracked(as_trackable(anchor)) }
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

/// Scene reconstruction: the provider whose anchors are the room's triangles.
///
/// The third provider, and the reason the core has [`wxr::Session::meshes`] as well as `planes`: a plane is a
/// surface ARKit recognised and a mesh is the surface itself. The shape is the plane provider's exactly - ask
/// for the anchors, read them - which is what makes this a leg rather than a rework.
struct Meshes {
    provider: Retained<ar_scene_reconstruction_provider_t>,
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
    meshes: Option<Meshes>,
    /// Light estimation, and the one provider here that describes the room by its light rather than its
    /// shape. Separate from the two above for the same reason they are separate from each other: a session
    /// can trace a room as triangles without ever being able to photograph it.
    lights: Option<Lights>,
    /// The world anchors this session made, by the name the core carries: an anchor is a handle, and
    /// the handle has to be able to find the object again.
    anchors: HashMap<u32, Retained<ar_world_anchor_t>>,
    next_anchor: u32,
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
            let meshes = Self::start_meshes(&providers);
            let lights = Lights::start(&providers);
            // None of the five is the only way there is no session. A device that can see the room or its
            // light but not track a head still has something to describe, and dropping the session because
            // the two providers this backend happens to need for a pose failed would throw that away too.
            if world.is_none()
                && hands.is_none()
                && planes.is_none()
                && meshes.is_none()
                && lights.is_none()
            {
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
                meshes,
                lights,
                anchors: HashMap::new(),
                next_anchor: 0,
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

    /// # Safety
    ///
    /// `providers` must be a live collection.
    unsafe fn start_meshes(providers: &ar_data_providers_t) -> Option<Meshes> {
        // SAFETY: as above. Support is asked first, because a provider on a device that has none is a
        // provider whose every query fails.
        unsafe {
            if !ar_scene_reconstruction_provider_t::is_supported() {
                return None;
            }
            let configuration = ar_scene_reconstruction_configuration_t::new();
            let provider = ar_scene_reconstruction_provider_t::new(&configuration);
            ar_data_providers_t::add_data_provider(providers, as_data_provider(&provider));
            Some(Meshes { provider })
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

    /// Whether scene reconstruction came up, which is what `features` asks and what `meshes` answers with.
    pub fn has_meshes(&self) -> bool {
        self.meshes.is_some()
    }

    /// The room's triangles, which is where `meshes` gets them.
    ///
    /// A copy and not a cache, for the same reason `planes` is one: ARKit re-traces a room as it learns more
    /// about it, and a stale copy of a wall that moved is the one thing a mesh is meant not to be.
    pub fn meshes(&self) -> Vec<wxr::Mesh> {
        let Some(meshes) = &self.meshes else {
            return Vec::new();
        };
        // SAFETY: the provider is live, and the enumerator only reads the anchors it is handed.
        unsafe {
            let anchors = ar_scene_reconstruction_provider_t::all_mesh_anchors(&meshes.provider);
            let mut out = Vec::new();
            let context = std::ptr::from_mut(&mut out).cast::<c_void>();
            ar_mesh_anchors_t::enumerate_anchors_f(&anchors, context, collect_mesh);
            out
        }
    }

    /// Whether the room's light can be estimated, which is what `features` asks.
    ///
    /// The provider being up, and not a promise of a photograph: a room ARKit cannot see well enough to
    /// light is a room that answers with no intensity and zero harmonics.
    pub fn has_light(&self) -> bool {
        self.lights.is_some()
    }

    /// What the room's light is, or `None` for the frames before ARKit has seen a lit one.
    ///
    /// A copy, like every other read here, and the cheapest thing in this file to call: the projection
    /// happened on ARKit's queue when the photograph arrived, and all this does is take a lock.
    pub fn light(&self) -> Option<wxr::LightEstimate> {
        self.lights.as_ref()?.estimate()
    }
}

/// Whether the platform reports that plane detection is supported.
pub fn planes_are_supported() -> bool {
    unsafe { objc2_ar_kit::ar_plane_detection_provider_t::is_supported() }
}
