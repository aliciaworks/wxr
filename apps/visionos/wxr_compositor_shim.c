// The CompositorServices and ARKit calls whose C type is a `simd` vector, written in the one place that ABI
// can be expressed exactly.
//
// `simd_float4x4` is returned in four SIMD registers (q0-q3), `simd_float4` in one (q0), and `simd_float2`
// is passed in one (d0). Rust cannot name any of those in a stable `extern "C"` signature - `simd_ffi` is
// unstable - and a `[f32; N]` struct uses a different ABI: it is returned via `x8`/in `s0-s3` and passed in
// separate 32-bit registers, so a declaration written in `sys.rs` would compile, link, and read the wrong
// register - a zero or uninitialised memory, not an error. Clang, compiling against Apple's own headers,
// gets these right, and the plain `float`/`float[16]` signatures on either side of this shim are the ABI
// Rust can speak.

#include <ARKit/ARKit.h>
#include <CompositorServices/CompositorServices.h>
#include <simd/simd.h>
#include <string.h>

// MARK: - CompositorServices

// Writes the transform from a view's space to the device's into `out`.
void wxr_cp_view_get_transform(cp_view_t view, float out[16]) {
    simd_float4x4 transform = cp_view_get_transform(view);
    memcpy(out, &transform, sizeof(transform));
}

// Writes the projection matrix for one of a drawable's views into `out`.
void wxr_cp_drawable_compute_projection(cp_drawable_t drawable,
                                        cp_axis_direction_convention convention,
                                        size_t view_index,
                                        float out[16]) {
    simd_float4x4 projection = cp_drawable_compute_projection(drawable, convention, view_index);
    memcpy(out, &projection, sizeof(projection));
}

// Sets a drawable's reverse-Z depth range, far component first - the order its own getter reads.
void wxr_cp_drawable_set_depth_range(cp_drawable_t drawable, float far, float near) {
    cp_drawable_set_depth_range(drawable, simd_make_float2(far, near));
}

// MARK: - ARKit

// Writes the transform from an anchor's space to the origin's into `out`.
void wxr_ar_anchor_get_origin_from_anchor_transform(ar_anchor_t anchor, float out[16]) {
    simd_float4x4 transform = ar_anchor_get_origin_from_anchor_transform(anchor);
    memcpy(out, &transform, sizeof(transform));
}

// Writes where a hand anchor is into `out`.
void wxr_ar_hand_anchor_get_origin_from_anchor_transform(ar_hand_anchor_t anchor, float out[16]) {
    simd_float4x4 transform = ar_hand_anchor_get_origin_from_anchor_transform(anchor);
    memcpy(out, &transform, sizeof(transform));
}

// Returns a world anchor the runtime will keep at the transform passed in `in`.
ar_world_anchor_t wxr_ar_world_anchor_create_with_origin_from_anchor_transform(const float in[16]) {
    simd_float4x4 transform;
    memcpy(&transform, in, sizeof(transform));
    return ar_world_anchor_create_with_origin_from_anchor_transform(transform);
}

// Writes where a world anchor is now into `out`.
void wxr_ar_world_anchor_get_origin_from_anchor_transform(ar_world_anchor_t anchor, float out[16]) {
    simd_float4x4 transform = ar_world_anchor_get_origin_from_anchor_transform(anchor);
    memcpy(out, &transform, sizeof(transform));
}
