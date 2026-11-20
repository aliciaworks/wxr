//! ARKit's plane anchors as the core's [`wxr::Plane`], and the enumerator that carries them across.
//!
//! Separate from the provider for the same reason [`super::mesh`] is: the provider is where the anchors
//! come from and this is what a plane anchor is once it is Rust's - a pose, an extent, and a horizontal or
//! a vertical. Nothing here asks ARKit a question; it answers one.

use std::ffi::c_void;
use std::time::Duration;

use objc2_ar_kit::{
    ar_plane_alignment_t, ar_plane_anchor_t, ar_plane_extent_t, ar_plane_geometry_t,
    ar_surface_classification_t,
};
use wxr::glam::Mat4;

use crate::sys;

/// One plane anchor, as the core's [`wxr::Plane`].
///
/// The pose is where the rectangle is, which is not quite where the anchor is: ARKit puts an anchor's origin
/// at the point it first saw the surface and grows the rectangle around it afterwards, so the extent carries
/// its own transform into the anchor's space. Composing the two gives the rectangle's centre and its turn,
/// with the surface's normal on +Y and the extent along X and Z - the space WebXR's plane space is, and the
/// one the core says a plane's pose is in.
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
        let geometry = ar_plane_anchor_t::geometry(anchor);
        let extent = ar_plane_geometry_t::plane_extent(&geometry);
        let anchor_from_extent = sys::ar_plane_extent_get_plane_anchor_from_plane_extent_transform(
            std::ptr::from_ref(&*extent).cast::<c_void>(),
        );
        let mut identifier = [0u8; 16];
        ar_plane_anchor_t::identifier(anchor, &mut identifier);
        let alignment = ar_plane_anchor_t::alignment(anchor);
        wxr::Plane {
            // The anchor's UUID folded to the name the core carries: the same 32 bits for as long as the
            // anchor lives, which is what a stable id has to be.
            id: u32::from_le_bytes([identifier[0], identifier[1], identifier[2], identifier[3]]),
            pose: rectangle_pose(transform, anchor_from_extent),
            extent: wxr::glam::Vec2::new(
                ar_plane_extent_t::width(&extent),
                ar_plane_extent_t::height(&extent),
            ),
            orientation: match alignment {
                a if a == ar_plane_alignment_t::horizontal => wxr::PlaneOrientation::Horizontal,
                a if a == ar_plane_alignment_t::vertical => wxr::PlaneOrientation::Vertical,
                _ => wxr::PlaneOrientation::Unknown,
            },
            // The anchor's classification rather than the geometry's: the geometry is the shape, and what
            // the shape is a picture of is the anchor's answer. `surface_classification` is the call Apple
            // points the older `plane_classification` at; the two lists differ only in the newer one adding
            // the labels the core already has.
            kind: kind(ar_plane_anchor_t::surface_classification(anchor)),
            // The runtime's clock, in seconds - the same number and the same meaning as a mesh's, because
            // ARKit timestamps the anchor and not the geometry.
            last_changed: Some(Duration::from_secs_f64(
                ar_plane_anchor_t::timestamp(anchor).max(0.0),
            )),
        }
    }
}

/// Where a plane's rectangle is: the anchor's transform composed with the extent's transform inside it.
///
/// Two matrices because they answer two questions - where the anchor is, and where the rectangle has moved
/// since - and the composition is the second applied first, which is what `origin_from_anchor` followed by
/// `anchor_from_extent` means. The extent transform is a translation, a turn about the surface normal, or
/// both; either way the normal stays on +Y and the rectangle's edges stay on the pose's X and Z.
fn rectangle_pose(
    origin_from_anchor: sys::Float4x4,
    anchor_from_extent: sys::Float4x4,
) -> wxr::Pose {
    let (_, orientation, position) = (Mat4::from_cols_array(&origin_from_anchor.0)
        * Mat4::from_cols_array(&anchor_from_extent.0))
    .to_scale_rotation_translation();
    wxr::Pose {
        position,
        orientation,
    }
}

/// What ARKit's classification of a plane is, in the core's terms.
///
/// `none` is both "not classified" and "not any of these", which the core says the same way: a caller that
/// has a surface to place something on does not need it to be a table. The richer labels - stairs, a bed, a
/// cabinet - are the ones the core learned from scene reconstruction, and a plane is rarely one of those.
fn kind(classification: ar_surface_classification_t) -> wxr::SurfaceKind {
    match classification {
        c if c == ar_surface_classification_t::wall => wxr::SurfaceKind::Wall,
        c if c == ar_surface_classification_t::floor => wxr::SurfaceKind::Floor,
        c if c == ar_surface_classification_t::ceiling => wxr::SurfaceKind::Ceiling,
        c if c == ar_surface_classification_t::table => wxr::SurfaceKind::Table,
        c if c == ar_surface_classification_t::seat => wxr::SurfaceKind::Seat,
        c if c == ar_surface_classification_t::window => wxr::SurfaceKind::Window,
        c if c == ar_surface_classification_t::door => wxr::SurfaceKind::Door,
        c if c == ar_surface_classification_t::cabinet => wxr::SurfaceKind::Cabinet,
        c if c == ar_surface_classification_t::bed => wxr::SurfaceKind::Bed,
        c if c == ar_surface_classification_t::plant => wxr::SurfaceKind::Plant,
        c if c == ar_surface_classification_t::stairs => wxr::SurfaceKind::Stairs,
        c if c == ar_surface_classification_t::tv => wxr::SurfaceKind::Tv,
        c if c == ar_surface_classification_t::home_appliance => wxr::SurfaceKind::HomeAppliance,
        _ => wxr::SurfaceKind::None,
    }
}

/// The enumerator `planes` hands to ARKit: one plane into the `Vec` the context points at.
///
/// # Safety
///
/// `context` must be the `*mut Vec<wxr::Plane>` the caller passed, and `anchor` a live plane anchor.
pub(super) unsafe extern "C-unwind" fn collect_plane(
    context: *mut c_void,
    anchor: &ar_plane_anchor_t,
) -> bool {
    // SAFETY: the caller guarantees both.
    unsafe {
        // A plane anchor ARKit has lost has no pose worth reporting, the same rule the other two backends
        // apply to a surface they cannot place this frame. It stays in the anchor list, so skipping it here
        // is what keeps a stale copy out of the frame.
        if !super::is_tracked(anchor) {
            return true;
        }
        (*context.cast::<Vec<wxr::Plane>>()).push(plane_of(anchor));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A transform that moves and turns nothing, which is where a plane starts.
    fn identity() -> sys::Float4x4 {
        sys::Float4x4(Mat4::IDENTITY.to_cols_array())
    }

    /// A plane that has not grown is still at the anchor.
    #[test]
    fn a_plane_at_its_anchor_has_the_anchor_pose() {
        assert_eq!(rectangle_pose(identity(), identity()), wxr::Pose::IDENTITY);
    }

    /// The extent's translation is where the rectangle has moved to, so the pose follows it.
    #[test]
    fn a_grown_plane_is_where_its_rectangle_is() {
        let anchor = Mat4::from_translation(wxr::glam::Vec3::new(1.0, 0.0, 2.0));
        let extent = Mat4::from_translation(wxr::glam::Vec3::new(0.5, 0.0, -0.25));
        let pose = rectangle_pose(
            sys::Float4x4(anchor.to_cols_array()),
            sys::Float4x4(extent.to_cols_array()),
        );
        let expected = wxr::glam::Vec3::new(1.5, 0.0, 1.75);
        assert!((pose.position - expected).length() < 1e-6);
    }

    /// A rectangle turned on the surface keeps the normal up and turns its own edges with it.
    #[test]
    fn an_extent_turned_about_the_normal_turns_the_pose() {
        let turned = Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let pose = rectangle_pose(identity(), sys::Float4x4(turned.to_cols_array()));
        assert!((pose.orientation * wxr::glam::Vec3::Y - wxr::glam::Vec3::Y).length() < 1e-6);
        assert!((pose.orientation * wxr::glam::Vec3::X - wxr::glam::Vec3::NEG_Z).length() < 1e-6);
    }
}
