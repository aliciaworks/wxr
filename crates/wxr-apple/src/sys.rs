//! The C functions that neither binding crate declares.
//!
//! `objc2-compositor-services` is generated from Apple's headers and binds most of CompositorServices - but
//! not the three calls this backend needs to place an eye. `cp_view_get_transform`, `cp_view_get_tangents`
//! and `cp_drawable_set_device_anchor` are real: Apple's own C guide,
//! [Drawing fully immersive content using Metal](https://developer.apple.com/documentation/compositorservices/drawing-fully-immersive-content-using-metal),
//! calls all three, and WebKit soft-links `cp_view_get_transform` for exactly the reason this file exists.
//! So they are declared here from those signatures, with the note that says why they are not in the crate
//! above them.
//!
//! ARKit is the other half. Its visionOS *Swift* API - `ARKitSession`, `WorldTrackingProvider` - has no
//! Objective-C presence at all, which is why `objc2` cannot reach it and why this workspace first thought a
//! Swift shim was needed. It is not: ARKit's **C API** is a separate, complete surface built for exactly
//! this case - "a C or C++ rendering engine" - with a session, providers, anchors, hand tracking and its own
//! `ar_release`. That is what is declared below, and it is the whole of what the tracking half needs.
//!
//! The `simd` types are the awkward part, and the alignment is why. `simd_float4x4` is sixteen floats that
//! Apple aligns to sixteen bytes, and a by-value return of one is written by Apple's code with SIMD stores
//! into a slot assumed to be aligned that way. A Rust `[[f32; 4]; 4]` has an alignment of four and a return
//! slot that may be four-byte aligned - which is a crash waiting for a device, and the sort of thing that is
//! invisible until it is not. Hence [`Float4x4`] and [`Float4`], which are `repr(C)` and aligned the way the
//! C type they stand for is.

use std::ffi::c_void;

use objc2_compositor_services::{cp_drawable_t, cp_view_t};

/// `simd_float4x4`: four columns of four floats, sixteen-byte aligned, column-major as `simd` stores them.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Float4x4(pub [f32; 16]);

/// `simd_float4`: four floats, sixteen-byte aligned.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Float4(pub [f32; 4]);

/// An ARKit object, as the C API sees it: a reference-counted object whose class this crate never needs to
/// name. Every `*_create` hands back one of these, and `ar_release` is what gives it back.
pub type ArSession = *mut c_void;
pub type ArDataProviders = *mut c_void;
pub type ArDataProvider = *mut c_void;
pub type ArWorldTrackingConfiguration = *mut c_void;
pub type ArWorldTrackingProvider = *mut c_void;
pub type ArDeviceAnchor = *mut c_void;
pub type ArAnchor = *mut c_void;

/// `ar_device_anchor_query_status_success`.
///
/// The enum has two cases, success and failure, and a C enum's first case is zero. That is the whole of the
/// assumption, and it is worth writing down because Apple's documentation lists the cases without numbers.
pub const QUERY_SUCCESS: isize = 0;

// SAFETY: every declaration below is transcribed from Apple's C header or from Apple's own C guide, and the
// framework is linked rather than loaded by hand. What is *not* proven here is the ABI - a wrong signature
// would still compile - which is why the types are `repr(C)` and aligned as the C ones are, and why the
// calls are made only with pointers that came from the framework itself.
#[link(name = "CompositorServices", kind = "framework")]
unsafe extern "C-unwind" {
    /// The transform from the view's space to the device's: the eye's own place in device space.
    ///
    /// Apple's guide calls this `deviceFromView` and composes it as `originFromDevice * deviceFromView` for
    /// the world-from-eye transform - so it is a pose, not a view matrix, and it is *not* inverted here.
    pub fn cp_view_get_transform(view: cp_view_t) -> Float4x4;

    /// The four half-angle tangents of a view's opening, in the order left, right, up, down.
    pub fn cp_view_get_tangents(view: cp_view_t) -> Float4;

    /// The head position and orientation to apply to the frame, which the compositor uses to reproject it
    /// if the prediction the app made turns out to be off.
    pub fn cp_drawable_set_device_anchor(drawable: cp_drawable_t, device_anchor: ArDeviceAnchor);
}

#[link(name = "ARKit", kind = "framework")]
unsafe extern "C-unwind" {
    pub fn ar_session_create() -> ArSession;
    /// Runs a session with the providers given, which the session retains.
    pub fn ar_session_run(session: ArSession, data_providers: ArDataProviders);
    pub fn ar_data_providers_create() -> ArDataProviders;
    pub fn ar_data_providers_add_data_provider(
        providers: ArDataProviders,
        provider: ArDataProvider,
    );
    pub fn ar_world_tracking_configuration_create() -> ArWorldTrackingConfiguration;
    pub fn ar_world_tracking_provider_create(
        configuration: ArWorldTrackingConfiguration,
    ) -> ArWorldTrackingProvider;
    /// Fills `device_anchor` in with the pose ARKit predicts for `timestamp`, in seconds.
    pub fn ar_world_tracking_provider_query_device_anchor_at_timestamp(
        provider: ArWorldTrackingProvider,
        timestamp: f64,
        device_anchor: ArDeviceAnchor,
    ) -> isize;
    pub fn ar_device_anchor_create() -> ArDeviceAnchor;
    /// The transform from the anchor's space to the origin's - for a device anchor, where the head is in the
    /// world ARKit tracks.
    pub fn ar_anchor_get_origin_from_anchor_transform(anchor: ArAnchor) -> Float4x4;
    /// Releases one reference, the counterpart to every `*_create` above.
    pub fn ar_release(object: *mut c_void);
}
