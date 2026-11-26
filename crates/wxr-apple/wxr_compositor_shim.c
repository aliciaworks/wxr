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

// Reads a drawable's depth range back, far component first - the pair the compositor is using, which is the
// layer configuration's until an app sets one.
void wxr_cp_drawable_get_depth_range(cp_drawable_t drawable, float out[2]) {
    simd_float2 range = cp_drawable_get_depth_range(drawable);
    memcpy(out, &range, sizeof(range));
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

// Writes the transform from an environment probe anchor's space to ARKit's world origin into `out`.
//
// The probe's environment texture is a cube around this anchor, so the light estimate integrated from it is
// in the anchor's own space - which is what the core's `Session::light_probe_pose` hands to an app that wants
// the primary direction in its own frame.
void wxr_ar_environment_probe_anchor_get_origin_from_anchor_transform(
    ar_environment_probe_anchor_t anchor, float out[16]) {
    simd_float4x4 transform = ar_environment_probe_anchor_get_origin_from_anchor_transform(anchor);
    memcpy(out, &transform, sizeof(transform));
}

// Writes the transform from a plane extent's space to the plane anchor carrying it into `out`.
//
// A plane anchor's own transform puts the origin where ARKit first saw the plane; as the plane grows, the
// rectangle moves and turns under that origin. The extent's transform is where the rectangle now sits in the
// anchor's space, so the anchor's transform composed with this one is the rectangle's centre - which is the
// pose the core calls a plane's.
void wxr_ar_plane_extent_get_plane_anchor_from_plane_extent_transform(ar_plane_extent_t extent,
                                                                      float out[16]) {
    simd_float4x4 transform = ar_plane_extent_get_plane_anchor_from_plane_extent_transform(extent);
    memcpy(out, &transform, sizeof(transform));
}

// Writes the transform from a skeleton joint to the hand anchor that holds it into `out`.
//
// The same shape as the hand anchor's own transform above, and the same reason it is here: the function
// returns a `simd_float4x4` by value. It is also the call that decides whether this backend can offer hand
// tracking at all - the joint transforms ARKit's *C* API hands over are what a skeleton is made of, and
// without them the anchors would be a palm and nothing else.
void wxr_ar_skeleton_joint_get_anchor_from_joint_transform(ar_skeleton_joint_t joint, float out[16]) {
    simd_float4x4 transform = ar_skeleton_joint_get_anchor_from_joint_transform(joint);
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

// Writes an accessory's estimated velocity, in metres per second, into `out`.
//
// Three floats, not `sizeof(simd_float3)`: the SIMD type is sixteen bytes wide with a fourth lane that is
// padding, and copying the whole of it into a `float[3]` would write past the end.
void wxr_ar_accessory_anchor_get_velocity(ar_accessory_anchor_t anchor, float out[3]) {
    simd_float3 velocity = ar_accessory_anchor_get_velocity(anchor);
    memcpy(out, &velocity, 3 * sizeof(float));
}

// Writes an accessory's estimated angular velocity, in radians per second, into `out`.
void wxr_ar_accessory_anchor_get_angular_velocity(ar_accessory_anchor_t anchor, float out[3]) {
    simd_float3 velocity = ar_accessory_anchor_get_angular_velocity(anchor);
    memcpy(out, &velocity, 3 * sizeof(float));
}

// Writes the transform from an accessory anchor's space to the origin's into `out`, corrected the way the
// caller asked: `none` is where the accessory actually is, and `rendered` is where to draw it so it lands
// over the physical object in passthrough.
void wxr_ar_accessory_anchor_get_origin_from_anchor_transform_with_correction(
    ar_accessory_anchor_t anchor, ar_transform_correction_t correction, float out[16]) {
    simd_float4x4 transform =
        ar_accessory_anchor_get_origin_from_anchor_transform_with_correction(anchor, correction);
    memcpy(out, &transform, sizeof(transform));
}

// Writes where one of an accessory's named locations is, in the anchor's space, into `out`.
void wxr_ar_accessory_anchor_get_anchor_from_location_transform_with_correction(
    ar_accessory_anchor_t anchor, const char *location_name, ar_transform_correction_t correction,
    float out[16]) {
    simd_float4x4 transform =
        ar_accessory_anchor_get_anchor_from_location_transform_with_correction(anchor, location_name,
                                                                              correction);
    memcpy(out, &transform, sizeof(transform));
}

// An accessory's name as ARKit gives it, or NULL when it has none - the header allows that, where the
// generated binding was taught the pointer is never null and would panic on the answer it documents.
const char *wxr_ar_accessory_get_name(ar_accessory_t accessory) {
    return ar_accessory_get_name(accessory);
}
