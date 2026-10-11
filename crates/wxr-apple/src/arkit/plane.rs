//! ARKit's plane anchors as the core's [`wxr::Plane`], and the enumerator that carries them across.
//!
//! Separate from the provider for the same reason [`super::mesh`] is: the provider is where the anchors
//! come from and this is what a plane anchor is once it is Rust's - a pose, an extent, and a horizontal or
//! a vertical. Nothing here asks ARKit a question; it answers one.

use std::ffi::c_void;

use objc2_ar_kit::{
    ar_plane_alignment_t, ar_plane_anchor_t, ar_plane_extent_t, ar_plane_geometry_t,
};
use wxr::glam::Mat4;

use crate::sys;

/// One plane anchor, as the core's [`wxr::Plane`].
///
/// The pose is the anchor's transform, which ARKit builds with the surface's normal on +Y and its extent
/// along X and Z - the same space WebXR's plane space is, and the one the core says a plane's pose is in.
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
        let (_, orientation, position) =
            Mat4::from_cols_array(&transform.0).to_scale_rotation_translation();
        let geometry = ar_plane_anchor_t::geometry(anchor);
        let extent = ar_plane_geometry_t::plane_extent(&geometry);
        let mut identifier = [0u8; 16];
        ar_plane_anchor_t::identifier(anchor, &mut identifier);
        let alignment = ar_plane_anchor_t::alignment(anchor);
        wxr::Plane {
            // The anchor's UUID folded to the name the core carries: the same 32 bits for as long as the
            // anchor lives, which is what a stable id has to be.
            id: u32::from_le_bytes([identifier[0], identifier[1], identifier[2], identifier[3]]),
            pose: wxr::Pose {
                position,
                orientation,
            },
            extent: wxr::glam::Vec2::new(
                ar_plane_extent_t::width(&extent),
                ar_plane_extent_t::height(&extent),
            ),
            orientation: match alignment {
                a if a == ar_plane_alignment_t::horizontal => wxr::PlaneOrientation::Horizontal,
                a if a == ar_plane_alignment_t::vertical => wxr::PlaneOrientation::Vertical,
                _ => wxr::PlaneOrientation::Unknown,
            },
        }
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
        (*context.cast::<Vec<wxr::Plane>>()).push(plane_of(anchor));
    }
    true
}
