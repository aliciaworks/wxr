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
//! ARKit is the other half, and now almost all of it comes from the generated `objc2-ar-kit` crate - ARKit
//! read from the SDK that has its visionOS C module. The two left here are the ones whose signature
//! *carries* a `simd` type: both return a `simd_float4x4` by value, and `objc2`'s translator cannot express
//! `simd` in a function yet. That by-value return is also why the types are `repr(C)` and aligned: Apple
//! writes one with SIMD stores into a slot assumed to be sixteen-byte aligned, and a Rust `[[f32; 4]; 4]`
//! has an alignment of four - a crash waiting for a device, and the sort of thing that is invisible until
//! it is not.

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

/// An ARKit anchor, as the hand-written calls below take it: the opaque pointer the generated crate hands
/// out, with the object it points at left to the type that owns it.
pub type ArDeviceAnchor = *mut c_void;

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
    /// The transform from the anchor's space to the origin's - for a device anchor, where the head is in the
    /// world ARKit tracks.
    ///
    /// The anchor is a pointer to one of the generated crate's anchors; it is `*const c_void` here because
    /// the return type is the reason the call is hand-written, not the argument.
    pub fn ar_anchor_get_origin_from_anchor_transform(anchor: *const c_void) -> Float4x4;

    /// Where the hand is.
    ///
    /// Apple's documentation has a page for
    /// `ar_hand_anchor_get_origin_from_anchor_transform_with_correction`, and visionOS 26.5's ARKit has
    /// no such symbol: the headers carry this one and nothing with "correction" in it, and a link against
    /// the SDK says `symbol(s) not found`. What the correction was for is a transform moved so that
    /// content drawn at it lands over the physical object in passthrough - input wants where the hand
    /// *is*, so the plain call is the one this backend wanted all along.
    pub fn ar_hand_anchor_get_origin_from_anchor_transform(hand_anchor: *const c_void) -> Float4x4;
}
