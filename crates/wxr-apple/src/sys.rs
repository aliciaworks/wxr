//! The C functions that neither binding crate declares.
//!
//! `objc2-compositor-services` is generated from Apple's headers and binds most of CompositorServices - but
//! not the four calls this backend needs to place an eye. `cp_view_get_transform`, `cp_view_get_tangents`,
//! `cp_drawable_set_depth_range` and `cp_drawable_set_device_anchor` are real: Apple's own C guide,
//! [Drawing fully immersive content using Metal](https://developer.apple.com/documentation/compositorservices/drawing-fully-immersive-content-using-metal),
//! calls them, and WebKit soft-links `cp_view_get_transform` for exactly the reason this file exists. So
//! they are declared here from those signatures, with the note that says why they are not in the crate
//! above them.
//!
//! **Nothing here is from memory.** [`Tools/check_apple_sys.py`](https://github.com/aliciaworks/wxr/blob/main/Tools/check_apple_sys.py) reads Apple's headers out of an SDK mirror
//! - the same headers an extractor without extended-attribute support reads as empty, which is what made
//!   this file unverifiable for a while - and compares each name, its argument count and the shape of what it
//!   returns; where the mirror has no header for something, WebKit's soft-link headers and Apple's
//!   documentation do. It runs in CI, and that is the whole of what makes a file of `extern "C"` a thing to
//!   keep rather than a thing to fear: a wrong signature compiles here and fails on a device, and there is no
//!   device here.
//!
//! One of the four is a call Apple has moved on from, and its header says so in words: `cp_view_get_tangents`
//! is `API_DEPRECATED("Use cp_drawable_compute_projection instead", visionos(1.0, 2.0))` and unavailable on
//! macOS. The replacement is in `objc2-compositor-services` already, so this is the one declaration here
//! that is expected to be *deleted* rather than corrected, and `projection` is where that swap would land.
//!
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

//! **Where the line between generated and written falls, and why.** Once the visionOS SDK was on hand,
//! these were run through `objc2`'s own translator, and the answer is *sixteen* of the twenty-one:
//! everything whose signature is plain C, plus `cp_drawable_set_device_anchor` once two lines that
//! skipped it were removed - they were there because ARKit is generated from the iOS SDK, which has no
//! visionOS module, and that is not true of this configuration.
//!
//! The five left over are the ones whose signature carries a `simd` type: four returning `simd_float4x4`
//! and one taking `simd_float2`. The translator says so itself - "simd types are not yet possible in
//! functions" - so the split is a *boundary* rather than a preference, and it is written down because a
//! reader would otherwise have to guess which declaration came from where.
//!
//! To redo it: `aliciaworks/objc2`, branch `visionos-arkit` - a one-line fix (an attribute macro with no
//! argument was read as one with), and `objc2-ar-kit`'s config naming `visionos` alone so that framework is
//! read from the SDK that has the module. The workflow `generate-visionos.yml` runs there on a macOS
//! runner with Xcode 26.6, one framework per run, because a run rewrites its crate's feature list and the
//! second run then cannot resolve the workspace. It was tried on Linux first and does not work there: the
//! libclang to hand is years newer than the one the translator was written against.

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

/// `simd_float2`: two floats, eight-byte aligned.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Float2(pub [f32; 2]);

/// An ARKit object, as the C API sees it: a reference-counted object whose class this crate never needs to
/// name. Every `*_create` hands back one of these, and `ar_release` is what gives it back.
pub type ArSession = *mut c_void;
pub type ArDataProviders = *mut c_void;
pub type ArDataProvider = *mut c_void;
pub type ArWorldTrackingConfiguration = *mut c_void;
pub type ArWorldTrackingProvider = *mut c_void;
pub type ArDeviceAnchor = *mut c_void;
pub type ArAnchor = *mut c_void;
pub type ArTrackableAnchor = *mut c_void;
pub type ArHandTrackingConfiguration = *mut c_void;
pub type ArHandTrackingProvider = *mut c_void;
pub type ArHandAnchor = *mut c_void;

/// `ar_device_anchor_query_status_success`.
///
/// The enum has two cases, success and failure, and a C enum's first case is zero. That is the whole of the
/// assumption, and it is worth writing down because Apple's documentation lists the cases without numbers.
/// On AArch64 a small enum is interchangeable between `int` and `NSInteger` in a register - a write to the
/// 32-bit half clears the 64-bit one - so declaring these as `isize` cannot misread a value that fits in
/// both, which every one of these does.
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

    /// The near and far planes the app drew with, so that the compositor can use the drawable's own depth
    /// buffer to reproject the frame.
    ///
    /// The pair is a `simd_float2` whose *first* component is the far plane and whose second is the near -
    /// which reads like a mistake and is not: the depth a `CompositorServices` drawable wants is reverse-Z,
    /// where nearer is the larger value. Apple's own guide reads the getter's pair the same way round
    /// (`depth_range[0]` far, `depth_range[1]` near), and a setter that disagreed with its own getter would be
    /// a trap.
    pub fn cp_drawable_set_depth_range(drawable: cp_drawable_t, depth_range: Float2);
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
    /// Whether ARKit is currently tracking this anchor. An untracked hand is still a hand with a pose, and
    /// a source that forgets that teleports.
    pub fn ar_trackable_anchor_is_tracked(anchor: ArTrackableAnchor) -> bool;

    pub fn ar_hand_tracking_configuration_create() -> ArHandTrackingConfiguration;
    pub fn ar_hand_tracking_provider_create(
        configuration: ArHandTrackingConfiguration,
    ) -> ArHandTrackingProvider;
    /// Whether this device can track hands at all, which is asked before a provider is made because a
    /// provider that is not supported is a provider whose queries all fail.
    pub fn ar_hand_tracking_provider_is_supported() -> bool;
    /// Fills both anchors in with the latest for each hand, and says whether it did.
    ///
    /// The two are handed over *left and right, in that order*, which is where a hand's handedness comes
    /// from - so no chirality enum has to be guessed at.
    pub fn ar_hand_tracking_provider_get_latest_anchors(
        provider: ArHandTrackingProvider,
        hand_anchor_left: ArHandAnchor,
        hand_anchor_right: ArHandAnchor,
    ) -> bool;
    pub fn ar_hand_anchor_create() -> ArHandAnchor;
    /// Where the hand is.
    ///
    /// Apple's documentation has a page for
    /// `ar_hand_anchor_get_origin_from_anchor_transform_with_correction`, and visionOS 26.5's ARKit has
    /// no such symbol: the headers carry this one and nothing with "correction" in it, and a link against
    /// the SDK says `symbol(s) not found`. What the correction was for is a transform moved so that
    /// content drawn at it lands over the physical object in passthrough - input wants where the hand
    /// *is*, so the plain call is the one this backend wanted all along.
    pub fn ar_hand_anchor_get_origin_from_anchor_transform(hand_anchor: ArHandAnchor) -> Float4x4;

    /// Releases one reference, the counterpart to every `*_create` above.
    pub fn ar_release(object: *mut c_void);
}
