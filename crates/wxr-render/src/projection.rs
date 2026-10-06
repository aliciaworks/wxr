//! Turning a field of view into a projection, and the two conventions there are.
//!
//! A field of view is four half-angles, because that is what a runtime reports and because a headset's lens
//! is not centred on its panel: the projection is asymmetric, and an approximation that centres it skews
//! everything the user sees.
//!
//! The trap in this file is the depth range. wgpu, Vulkan, D3D12 and Metal put it in `0..w`; OpenGL and
//! WebGL put it in `-w..w`, and **WebXR reports the second**. A projection taken from a browser and handed
//! to wgpu unaltered is an eye whose depth is half off - which looks like a scene that is all clipped, or
//! all drawn, depending on where the near plane landed. [`from_gl`] is the one matrix that fixes it,
//! [`angles`] is the way back, and [`Depth::Reverse`] is the third arrangement a platform can insist on.
//! CompositorServices is that platform: a pass drawn into a visionOS drawable uses reverse-Z or its depth is
//! the wrong way round, which is why the convention is named here rather than left to whoever draws.

use wxr::FieldOfView;
use wxr::glam::{Mat4, Vec3};

/// Where a projection puts its depth range.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Depth {
    /// `0..w`: wgpu, Vulkan, D3D12, Metal. What this crate draws with.
    #[default]
    ZeroToOne,
    /// `-w..w`: OpenGL, WebGL, and therefore WebXR.
    MinusOneToOne,
    /// `1..0`: nearer is larger. Reverse-Z, which is what a float depth buffer wants for its precision and
    /// what visionOS's compositor requires outright - a pass drawn into a `CompositorServices` drawable has
    /// to use it, and its own `cp_drawable_compute_projection` agrees.
    Reverse,
}

/// A projection from a field of view, for a camera that looks down its own `-Z`.
///
/// Right-handed, and the near and far planes are the only distances involved: the four openings come from
/// the angles, so a wider eye is a wider frustum rather than a scaled one.
pub fn perspective(fov: FieldOfView, near: f32, far: f32, depth: Depth) -> Mat4 {
    // The tangents of the half-angles are the frustum's edges at one unit out.
    let (left, right) = (fov.left.tan(), fov.right.tan());
    let (up, down) = (fov.up.tan(), fov.down.tan());
    let width = right + left;
    let height = up + down;
    if width <= 0.0 || height <= 0.0 || near <= 0.0 || far <= near {
        return Mat4::IDENTITY;
    }

    let scale_x = 2.0 / width;
    let scale_y = 2.0 / height;
    // Where the opening's centre is, as a fraction of its width. Zero is a centred projection, and it is
    // not zero on a headset.
    let offset_x = (right - left) / width;
    let offset_y = (up - down) / height;

    let (z_scale, z_offset) = match depth {
        Depth::ZeroToOne => (far / (near - far), far * near / (near - far)),
        Depth::MinusOneToOne => ((far + near) / (near - far), 2.0 * far * near / (near - far)),
        // The same two planes, met from the other end: the near plane at one and the far at zero. Solving
        // `-near * scale + offset = near` and `-far * scale + offset = 0` gives these, and it is the only
        // difference reverse-Z has.
        Depth::Reverse => (near / (far - near), far * near / (far - near)),
    };

    Mat4::from_cols_array(&[
        scale_x, 0.0, 0.0, 0.0, //
        0.0, scale_y, 0.0, 0.0, //
        offset_x, offset_y, z_scale, -1.0, //
        0.0, 0.0, z_offset, 0.0,
    ])
}

/// The field of view a projection was made from.
///
/// This is the way back, and it exists because WebXR hands out a matrix rather than angles: a backend for it
/// has to read the four openings out of the matrix, and a renderer that wants them has to have them.
pub fn angles(projection: Mat4) -> FieldOfView {
    let column = projection.z_axis;
    let (scale_x, scale_y) = (projection.x_axis.x, projection.y_axis.y);
    if scale_x == 0.0 || scale_y == 0.0 {
        return FieldOfView::symmetric(0.0, 0.0);
    }
    // The offsets are the third column's x and y, which is where `perspective` put them.
    let (offset_x, offset_y) = (column.x, column.y);
    FieldOfView {
        left: (-(offset_x - 1.0) / scale_x).atan(),
        right: ((offset_x + 1.0) / scale_x).atan(),
        up: ((offset_y + 1.0) / scale_y).atan(),
        down: (-(offset_y - 1.0) / scale_y).atan(),
    }
}

/// A field of view from its four half-angle tangents, which is how a compositor that reports tangents
/// reports it.
///
/// Apple's CompositorServices hands a view's opening over as `tangents` - the tangents of the four
/// half-angles, in the order left, right, up, down - and a tangent is a field of view with an `atan` in
/// front of it. It lives here rather than in the Apple backend because the arithmetic is not Apple's: a
/// tangent is a distance over a distance, and any runtime that reports one gets the same answer.
pub fn angles_from_tangents(tangents: [f32; 4]) -> FieldOfView {
    let [left, right, up, down] = tangents;
    FieldOfView {
        up: up.atan(),
        down: down.atan(),
        left: left.atan(),
        right: right.atan(),
    }
}

/// A projection in OpenGL's convention, as wgpu's.
///
/// The z row is remapped from `-w..w` to `0..w`, which is a scale of a half and a shift of a half in
/// homogeneous terms. Everything else is untouched: the x and y rows are the same convention on both sides.
pub fn from_gl(projection: Mat4) -> Mat4 {
    let to_zero_to_one = Mat4::from_cols_array(&[
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 0.5, 0.0, //
        0.0, 0.0, 0.5, 1.0,
    ]);
    to_zero_to_one * projection
}

/// A camera's view matrix from where it is: the inverse of the pose, because a camera is a place the world
/// is seen from rather than a thing in it.
pub fn view(pose: wxr::Pose) -> Mat4 {
    pose.transform().inverse().into()
}

/// A pose from a view matrix: the way back from [`view`], and the one a runtime that reports a camera
/// matrix instead of a place has to hand over.
pub fn pose_from_view(transform: Mat4) -> wxr::Pose {
    pose_from_transform(transform.inverse())
}

/// A pose from a transform that already is one.
///
/// The difference from [`pose_from_view`] is one inversion, and it matters: a view matrix is a camera's pose
/// *inverted*, and a runtime that reports an eye's place in device space - CompositorServices'
/// `cp_view_get_transform`, which Apple's guide calls `deviceFromView` - has already done the inversion for
/// us. Inverting that one again would put every eye behind the wearer.
pub fn pose_from_transform(transform: Mat4) -> wxr::Pose {
    let (_, orientation, position) = transform.to_scale_rotation_translation();
    wxr::Pose {
        position,
        orientation: orientation.normalize(),
    }
}

/// A direction in the world from a camera that looks down its own `-Z`.
pub fn forward(pose: wxr::Pose) -> Vec3 {
    pose.orientation * Vec3::NEG_Z
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn angles_and_the_matrix_are_the_same_fact() {
        let fov = FieldOfView {
            up: 0.5,
            down: 0.4,
            left: 0.6,
            right: 0.55,
        };
        let back = angles(perspective(fov, 0.05, 100.0, Depth::ZeroToOne));
        assert!(close(back.up, fov.up), "{back:?}");
        assert!(close(back.down, fov.down), "{back:?}");
        assert!(close(back.left, fov.left), "{back:?}");
        assert!(close(back.right, fov.right), "{back:?}");
    }

    #[test]
    fn a_centred_field_of_view_is_a_centred_projection() {
        let fov = FieldOfView::symmetric(FRAC_PI_2, FRAC_PI_2);
        let matrix = perspective(fov, 0.1, 10.0, Depth::ZeroToOne);
        assert!(matrix.z_axis.x.abs() < 1e-6, "no horizontal offset");
        assert!(matrix.z_axis.y.abs() < 1e-6, "no vertical offset");
        // A quarter turn is a square, and the projection of one is a unit scale.
        assert!(close(matrix.x_axis.x, 1.0));
        assert!(close(matrix.y_axis.y, 1.0));
    }

    #[test]
    fn an_offset_field_of_view_leans_the_projection_the_same_way() {
        let fov = FieldOfView {
            up: 0.5,
            down: 0.5,
            left: 0.0,
            right: FRAC_PI_4,
        };
        let matrix = perspective(fov, 0.1, 10.0, Depth::ZeroToOne);
        // Everything opens to the right, so the camera's own axis is the left edge of the picture: the
        // frustum's left edge is at x = 0 and its right at x = 1, and the offset is what puts them there.
        assert!(close(matrix.z_axis.x, 1.0), "{:?}", matrix.z_axis.x);
        // Which is the same statement as: the centre of the frustum lands in the centre of the picture.
        let centre = (fov.right.tan() - fov.left.tan()) * 0.5;
        let ndc = matrix.x_axis.x * centre - matrix.z_axis.x;
        assert!(ndc.abs() < 1e-5, "{ndc}");
    }

    #[test]
    fn the_near_plane_lands_on_zero_and_the_far_on_one() {
        // The one thing a depth range has to do, checked where it is used: the near plane in a point that is
        // `near` away, and the far plane in one that is `far` away. Both are on the -Z axis, where a camera
        // looks.
        let fov = FieldOfView::symmetric(FRAC_PI_2, FRAC_PI_2);
        let (near, far) = (0.5_f32, 20.0_f32);
        let matrix = perspective(fov, near, far, Depth::ZeroToOne);
        for (distance, expected) in [(near, 0.0), (far, 1.0)] {
            let clip = matrix * wxr::glam::Vec4::new(0.0, 0.0, -distance, 1.0);
            assert!(close(clip.z / clip.w, expected), "{distance}: {clip:?}");
        }
    }

    #[test]
    fn a_gl_projection_becomes_a_wgpu_one() {
        // The same camera, twice: once in the convention WebXR reports and once in the one wgpu wants. The
        // near and far planes have to land in the same places afterwards.
        let fov = FieldOfView::symmetric(FRAC_PI_2, FRAC_PI_2);
        let (near, far) = (0.5_f32, 20.0_f32);
        let gl = perspective(fov, near, far, Depth::MinusOneToOne);
        let converted = from_gl(gl);
        let ours = perspective(fov, near, far, Depth::ZeroToOne);
        for (distance, expected) in [(near, 0.0), (far, 1.0)] {
            let a = converted * wxr::glam::Vec4::new(0.0, 0.0, -distance, 1.0);
            let b = ours * wxr::glam::Vec4::new(0.0, 0.0, -distance, 1.0);
            assert!(close(a.z / a.w, expected), "{distance}: {a:?}");
            assert!(close(a.z / a.w, b.z / b.w), "{distance}");
        }
    }

    #[test]
    fn a_view_matrix_puts_the_camera_at_its_own_origin() {
        let pose = wxr::Pose {
            position: Vec3::new(1.0, 2.0, 3.0),
            orientation: wxr::glam::Quat::from_rotation_y(FRAC_PI_2),
        };
        let eye = view(pose) * pose.position.extend(1.0);
        assert!(eye.truncate().length() < 1e-5, "{eye:?}");
        // And it looks along the pose's own -Z, which after a quarter turn is -X in the world.
        assert!(
            forward(pose).abs_diff_eq(Vec3::NEG_X, 1e-5),
            "{:?}",
            forward(pose)
        );
    }

    #[test]
    fn tangents_are_angles_waiting_for_an_atan() {
        // An opening of a quarter turn each way has tangents of one, and the four directions come back as
        // the quarter turns they were.
        let half = FRAC_PI_4;
        let angles = angles_from_tangents([half.tan(), half.tan(), half.tan(), half.tan()]);
        assert!(close(angles.up, half), "{angles:?}");
        assert!(close(angles.left, half), "{angles:?}");
        // Asymmetric, and the asymmetry is in the tangents: four different openings stay four.
        let skew = angles_from_tangents([0.5, 1.5, 2.0, 0.25]);
        assert!(close(skew.left, 0.5_f32.atan()), "{skew:?}");
        assert!(close(skew.right, 1.5_f32.atan()), "{skew:?}");
        assert!(close(skew.up, 2.0_f32.atan()), "{skew:?}");
        assert!(close(skew.down, 0.25_f32.atan()), "{skew:?}");
    }

    #[test]
    fn a_view_matrix_is_a_pose_the_other_way_round() {
        let pose = wxr::Pose {
            position: Vec3::new(-0.03, 0.1, 0.2),
            orientation: wxr::glam::Quat::from_rotation_y(0.7)
                * wxr::glam::Quat::from_rotation_x(0.2),
        };
        let back = pose_from_view(view(pose));
        assert!(back.position.abs_diff_eq(pose.position, 1e-5), "{back:?}");
        // Compared by where they point rather than by their components: a rotation and its negation are
        // the same rotation, and a decomposition is free to hand back either.
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            assert!(
                (back.orientation * axis).abs_diff_eq(pose.orientation * axis, 1e-5),
                "{axis:?}: {back:?}"
            );
        }
    }

    #[test]
    fn a_transform_that_is_already_a_pose_is_not_inverted_again() {
        // The distinction that matters for CompositorServices: `deviceFromView` is a place, not a camera
        // matrix, so it goes through here and not through `pose_from_view`. An eye a hand's width to the
        // left has to come back a hand's width to the left.
        let eye = wxr::Pose {
            position: Vec3::new(-0.031, 0.0, 0.0),
            orientation: wxr::glam::Quat::from_rotation_y(0.1),
        };
        let back = pose_from_transform(eye.transform().into());
        assert!(back.position.abs_diff_eq(eye.position, 1e-5), "{back:?}");
        // And the other way round would put it on the wrong side, which is what this is guarding.
        assert!(pose_from_view(eye.transform().into()).position.x > 0.0);
    }

    #[test]
    fn a_reverse_projection_puts_the_near_plane_at_one() {
        // visionOS's convention, and the one thing it has to do: the same two planes, met from the other
        // end. A nearest thing in the near plane has to be the largest value the buffer can hold.
        let fov = FieldOfView::symmetric(FRAC_PI_2, FRAC_PI_2);
        let (near, far) = (0.5_f32, 20.0_f32);
        let matrix = perspective(fov, near, far, Depth::Reverse);
        for (distance, expected) in [(near, 1.0), (far, 0.0)] {
            let clip = matrix * wxr::glam::Vec4::new(0.0, 0.0, -distance, 1.0);
            assert!(close(clip.z / clip.w, expected), "{distance}: {clip:?}");
        }
    }
}
