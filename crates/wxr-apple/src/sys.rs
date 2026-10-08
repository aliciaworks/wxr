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

use objc2_compositor_services::{cp_axis_direction_convention, cp_drawable_t, cp_view_t};

/// `simd_float4x4`: four columns of four floats, sixteen-byte aligned, column-major as `simd` stores them.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Float4x4(pub [f32; 16]);

/// `simd_float2`: two floats, eight-byte aligned.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Float2(pub [f32; 2]);

/// An ARKit anchor, as the hand-written calls below take it: the opaque pointer the generated crate hands
/// out, with the object it points at left to the type that owns it.
pub type ArDeviceAnchor = *mut c_void;

/// A world anchor the same way, for the two calls that create one and place it.
pub type ArWorldAnchor = *mut c_void;

// SAFETY: every `simd`-typed call below is made through `apps/visionos/wxr_compositor_shim.c`, which clang
// compiles against the real headers, and the wrappers here are its plain-float side: a `simd` vector is
// passed and returned in SIMD registers and a stable Rust `extern` cannot say that, so no `simd` type is
// declared in this file at all. What is declared as C here is the set of pointer-and-scalar calls the
// wrapper does not need a shim for - `cp_drawable_set_device_anchor` - and the shim functions themselves.
//
// The wrappers are `pub unsafe fn` with bodies, not `pub fn ... ;` declarations, so
// `Tools/check_apple_sys.py` - which looks `pub fn` up in Apple's headers - reads only
// `cp_drawable_set_device_anchor` from this file. The Apple calls behind the shim are checked where the shim
// is compiled, by clang, against the same headers.

#[link(name = "CompositorServices", kind = "framework")]
unsafe extern "C-unwind" {
    /// The head position and orientation to apply to the frame, which the compositor uses to reproject it
    /// if the prediction the app made turns out to be off.
    ///
    /// A pointer argument and a pointer return, so it needs no shim.
    pub fn cp_drawable_set_device_anchor(drawable: cp_drawable_t, device_anchor: ArDeviceAnchor);
}

// The shim, which is `crates/wxr-apple/wxr_compositor_shim.c` in this crate but compiled by whatever
// links the app: it is C, and a `staticlib` does not carry an object a build script produced, so the
// step belongs to the final link - `apps/visionos/build.sh` here, and the same one in a consumer.
unsafe extern "C" {
    /// Writes the transform from a view's space to the device's into `out`.
    fn wxr_cp_view_get_transform(view: cp_view_t, out: *mut f32);
    /// Writes the projection matrix for one of a drawable's views into `out`.
    fn wxr_cp_drawable_compute_projection(
        drawable: cp_drawable_t,
        normalized_device_coordinates_convension: cp_axis_direction_convention,
        view_index: usize,
        out: *mut f32,
    );
    /// Sets a drawable's reverse-Z depth range, the far plane first.
    fn wxr_cp_drawable_set_depth_range(drawable: cp_drawable_t, far: f32, near: f32);

    /// Writes the transform from an anchor's space to the origin's into `out`.
    fn wxr_ar_anchor_get_origin_from_anchor_transform(anchor: *const c_void, out: *mut f32);
    /// Writes where a hand anchor is into `out`.
    fn wxr_ar_hand_anchor_get_origin_from_anchor_transform(
        hand_anchor: *const c_void,
        out: *mut f32,
    );
    /// Returns a world anchor the runtime will keep at the transform passed in `transform`.
    fn wxr_ar_world_anchor_create_with_origin_from_anchor_transform(
        transform: *const f32,
    ) -> ArWorldAnchor;
    /// Writes where a world anchor is now into `out`.
    fn wxr_ar_world_anchor_get_origin_from_anchor_transform(anchor: *const c_void, out: *mut f32);
}

/// The transform from the view's space to the device's: the eye's own place in device space.
///
/// Apple's guide calls this `deviceFromView` and composes it as `originFromDevice * deviceFromView` for
/// the world-from-eye transform - so it is a pose, not a view matrix, and it is *not* inverted here.
///
/// # Safety
///
/// `view` must be a live `cp_view_t`.
pub unsafe fn cp_view_get_transform(view: cp_view_t) -> Float4x4 {
    let mut out = [0.0f32; 16];
    // SAFETY: the caller's contract is the shim's, and `out` is sixteen contiguous `f32`s.
    unsafe { wxr_cp_view_get_transform(view, out.as_mut_ptr()) };
    Float4x4(out)
}

/// The projection matrix for one of a drawable's views.
///
/// This is what a mixed-reality layer wants instead of `cp_view_get_tangents`, which the compositor refuses
/// there outright ("For mixed reality experiences please use cp_drawable_compute_projection"). The matrix is
/// read back into the four openings by `wxr_render::angles`. `right_up_back` is the convention this renderer
/// draws with - right-handed, looking down -Z.
///
/// # Safety
///
/// `drawable` must be a live `cp_drawable_t` for the current frame.
pub unsafe fn cp_drawable_compute_projection(
    drawable: cp_drawable_t,
    normalized_device_coordinates_convension: cp_axis_direction_convention,
    view_index: usize,
) -> Float4x4 {
    let mut out = [0.0f32; 16];
    // SAFETY: the caller's contract is the shim's, and `out` is sixteen contiguous `f32`s.
    unsafe {
        wxr_cp_drawable_compute_projection(
            drawable,
            normalized_device_coordinates_convension,
            view_index,
            out.as_mut_ptr(),
        )
    };
    Float4x4(out)
}

/// The near and far planes the app drew with, so that the compositor can use the drawable's own depth
/// buffer to reproject the frame.
///
/// The pair is passed far first and near second, which reads like a mistake and is not: the depth a
/// `CompositorServices` drawable wants is reverse-Z, where nearer is the larger value. Apple's own guide
/// reads the getter's pair the same way round (`depth_range[0]` far, `depth_range[1]` near), and a setter
/// that disagreed with its own getter would be a trap. The near plane also has a floor: the compositor
/// refuses anything closer than 0.1 m.
///
/// # Safety
///
/// `drawable` must be a live `cp_drawable_t` for the current frame.
pub unsafe fn cp_drawable_set_depth_range(drawable: cp_drawable_t, depth_range: Float2) {
    // SAFETY: the caller's contract is the shim's.
    unsafe { wxr_cp_drawable_set_depth_range(drawable, depth_range.0[0], depth_range.0[1]) };
}

/// The transform from an anchor's space to the origin's - for a device anchor, where the head is in the
/// world ARKit tracks.
///
/// # Safety
///
/// `anchor` must be a live anchor object from the generated crate.
pub unsafe fn ar_anchor_get_origin_from_anchor_transform(anchor: *const c_void) -> Float4x4 {
    let mut out = [0.0f32; 16];
    // SAFETY: the caller's contract is the shim's, and `out` is sixteen contiguous `f32`s.
    unsafe { wxr_ar_anchor_get_origin_from_anchor_transform(anchor, out.as_mut_ptr()) };
    Float4x4(out)
}

/// Where the hand is.
///
/// Apple's documentation has a page for
/// `ar_hand_anchor_get_origin_from_anchor_transform_with_correction`, and visionOS 26.5's ARKit has no
/// such symbol: the headers carry this one and nothing with "correction" in it, and a link against the SDK
/// says `symbol(s) not found`. What the correction was for is a transform moved so that content drawn at it
/// lands over the physical object in passthrough - input wants where the hand *is*, so the plain call is the
/// one this backend wanted all along.
///
/// # Safety
///
/// `hand_anchor` must be a live hand anchor object from the generated crate.
pub unsafe fn ar_hand_anchor_get_origin_from_anchor_transform(
    hand_anchor: *const c_void,
) -> Float4x4 {
    let mut out = [0.0f32; 16];
    // SAFETY: the caller's contract is the shim's, and `out` is sixteen contiguous `f32`s.
    unsafe { wxr_ar_hand_anchor_get_origin_from_anchor_transform(hand_anchor, out.as_mut_ptr()) };
    Float4x4(out)
}

/// A world anchor the runtime will keep at this place - the `origin from anchor` transform of the anchor's
/// own space, which is the same shape every space in the core is.
///
/// # Safety
///
/// `origin_from_anchor_transform` is used only for its sixteen floats, and the object it returns is `+1`
/// like every other `ar_*`.
pub unsafe fn ar_world_anchor_create_with_origin_from_anchor_transform(
    origin_from_anchor_transform: Float4x4,
) -> ArWorldAnchor {
    // SAFETY: the caller's contract is the shim's, and the array is sixteen contiguous `f32`s.
    unsafe {
        wxr_ar_world_anchor_create_with_origin_from_anchor_transform(
            origin_from_anchor_transform.0.as_ptr(),
        )
    }
}

/// Where a world anchor is now.
///
/// The reason an anchor is a handle and not a pose: the runtime moves it as its understanding of the room
/// changes, and reading it is how an app finds out where its object ended up.
///
/// # Safety
///
/// `anchor` must be a live world anchor object from the generated crate.
pub unsafe fn ar_world_anchor_get_origin_from_anchor_transform(anchor: *const c_void) -> Float4x4 {
    let mut out = [0.0f32; 16];
    // SAFETY: the caller's contract is the shim's, and `out` is sixteen contiguous `f32`s.
    unsafe { wxr_ar_world_anchor_get_origin_from_anchor_transform(anchor, out.as_mut_ptr()) };
    Float4x4(out)
}
