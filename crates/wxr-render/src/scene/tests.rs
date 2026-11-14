//! The scene's own arithmetic, where a GPU is not what the question is about.
//!
//! The field of view has to survive the round trip through a projection, the two triangle depths have to stay
//! ordered, and the occlusion uniform has to be the bytes the shader reads - a mismatch there is a picture that
//! is wrong rather than a build that fails. Split out of `scene.rs` for the size rule: the scene itself is the
//! drawing, and this is the handful of facts it rests on.

use super::*;
use crate::projection::angles;

#[test]
fn the_default_field_of_view_survives_the_round_trip() {
    let back = angles(perspective(DEFAULT_FOV, 0.05, 100.0, Depth::ZeroToOne));
    assert!((back.up - DEFAULT_FOV.up).abs() < 1e-4);
    assert!((back.left - DEFAULT_FOV.left).abs() < 1e-4);
}

#[test]
fn a_triangle_at_the_far_plane_is_behind_a_triangle_at_the_near_one() {
    // The two depths the projection has to order, which is the one thing a depth range is for.
    let range = planes();
    let matrix = perspective(DEFAULT_FOV, range.near, range.far, Depth::ZeroToOne);
    let at = |z: f32| (matrix * wxr::glam::Vec4::new(0.0, 0.0, z, 1.0)).z;
    assert!(at(-range.near) < at(-range.far), "nearer is smaller");
}

#[test]
fn the_occlusion_uniform_is_what_the_shader_says_it_is() {
    // The shader's `Occlusion` is 64 bytes of matrix, then `kind` and `enabled` and the three numbers the two
    // encodings take - six 4-byte fields - and a spare plus a two-uint pad to carry it to the next 16-byte
    // block. A mismatch is a picture that is wrong rather than a build that fails, so it is pinned here, and
    // the field *order* is the shader's too: `kind` before the numbers it selects.
    assert_eq!(std::mem::size_of::<OcclusionUniform>(), 64 + 6 * 4 + 8);
    assert_eq!(
        std::mem::offset_of!(OcclusionUniform, kind),
        64,
        "the shader branches on `kind` immediately after the matrix"
    );
    assert_eq!(
        std::mem::offset_of!(OcclusionUniform, _pad),
        88,
        "the pad starts on an 8-byte boundary, which is what a vec2 needs"
    );
}
