//! ARKit's plane anchors as the core's [`wxr::Plane`], and the enumerator that carries them across.
//!
//! Separate from the provider for the same reason [`super::mesh`] is: the provider is where the anchors
//! come from and this is what a plane anchor is once it is Rust's - a pose, an extent, and a horizontal or
//! a vertical. Nothing here asks ARKit a question; it answers one.

use std::collections::HashMap;
use std::ffi::c_void;
use std::time::Duration;

use objc2_ar_kit::{
    ar_plane_alignment_t, ar_plane_anchor_t, ar_plane_extent_t, ar_plane_geometry_t,
    ar_surface_classification_t,
};
use wxr::glam::{Mat4, Vec2};

use crate::sys;

/// One plane anchor, as the core's [`wxr::Plane`].
///
/// The pose is where the rectangle is, which is not quite where the anchor is: ARKit puts an anchor's origin
/// at the point it first saw the surface and grows the rectangle around it afterwards, so the extent carries
/// its own transform into the anchor's space. Composing the two gives the rectangle's centre and its turn,
/// with the surface's normal on +Y and the extent along X and Z - the space WebXR's plane space is, and the
/// one the core says a plane's pose is in.
///
/// The outline is the plane's mesh, and it is not the rectangle: Apple describes the geometry as the convex
/// polygon around everything ARKit currently thinks is part of the surface, and the extent as the rectangle
/// that encloses it. The mesh's vertices are in the anchor's space, so each point is brought into the
/// rectangle's space before it is reported - the same transform the pose is built from, read the other way.
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
            // The mesh's rim, which is the polygon Apple says the geometry encloses, and not the extent's
            // four corners: an L-shaped table has six or more points.
            outline: outline_of(&geometry, anchor_from_extent),
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

/// The plane's boundary, as the core's outline, in the rectangle's space.
///
/// ARKit gives a plane as a triangle mesh and the extent as the rectangle around it; the boundary is the edges
/// no second triangle shares, walked in the direction the triangles wind them. Points come out of the mesh in
/// the anchor's space - Apple says the anchor's transform defines it - and the core's plane pose is the
/// rectangle's, so each point goes through the inverse of the extent's transform first.
///
/// `None` for a mesh this cannot read: no faces, a rim in more than one piece, or a walk that does not close.
/// A plane with no outline is one a caller draws from its extent, which is what every caller did before.
///
/// # Safety
///
/// `geometry` must be a live plane geometry.
unsafe fn outline_of(
    geometry: &ar_plane_geometry_t,
    anchor_from_extent: sys::Float4x4,
) -> Option<Vec<Vec2>> {
    // SAFETY: the geometry is live, and both buffers are the runtime's - the mesh readers have the same
    // contract they have for a mesh anchor.
    unsafe {
        let vertices = super::mesh::read_vectors(&ar_plane_geometry_t::mesh_vertices(geometry));
        let indices = super::mesh::read_indices(&ar_plane_geometry_t::mesh_faces(geometry));
        let rim = boundary(&indices, vertices.len())?;
        let extent_from_anchor = Mat4::from_cols_array(&anchor_from_extent.0).inverse();
        Some(
            rim.into_iter()
                .map(|index| {
                    let point = extent_from_anchor.transform_point3(vertices[index]);
                    // The extent's space is the core plane's: X across and Z along, with the normal on Y.
                    // A point that is a rounding error off the surface has its Y dropped rather than
                    // reported as a second shape a hair above the first.
                    Vec2::new(point.x, point.z)
                })
                .collect(),
        )
    }
}

/// The vertices on the rim of a triangle mesh, in the order the triangles walk them.
///
/// An edge inside the mesh is shared by two triangles and an edge on the rim by one, so counting the
/// undirected edges and keeping the ones that occurred once is the whole of the classification. The rim edges
/// keep their direction, and a consistently wound mesh gives every rim vertex one edge in and one edge out -
/// which is what turns the set into a loop.
///
/// `None` when the mesh is not one closed rim: an index past the vertices, a triangle with a repeated vertex,
/// a rim vertex with two edges out or two in, or a rim that does not come back to where it started.
fn boundary(indices: &[u32], vertices: usize) -> Option<Vec<usize>> {
    let (triangles, rest) = indices.as_chunks::<3>();
    if !rest.is_empty() {
        return None;
    }
    let mut used: HashMap<(u32, u32), usize> = HashMap::new();
    let mut directed = Vec::with_capacity(indices.len());
    for triangle in triangles {
        for (from, to) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            if from == to || from as usize >= vertices || to as usize >= vertices {
                return None;
            }
            *used.entry((from.min(to), from.max(to))).or_default() += 1;
            directed.push((from, to));
        }
    }
    let mut next: HashMap<u32, u32> = HashMap::new();
    let mut incoming: HashMap<u32, u32> = HashMap::new();
    for (from, to) in directed {
        if used[&(from.min(to), from.max(to))] != 1 {
            continue;
        }
        if next.insert(from, to).is_some() || incoming.insert(to, from).is_some() {
            return None;
        }
    }
    let start = *next.keys().min()?;
    let mut rim = Vec::with_capacity(next.len());
    let mut at = start;
    loop {
        rim.push(at as usize);
        at = *next.get(&at)?;
        if at == start {
            break;
        }
        if rim.len() > next.len() {
            return None;
        }
    }
    (rim.len() == next.len()).then_some(rim)
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

    /// A square of two triangles: the rim is the four corners, not the diagonal that both triangles share.
    #[test]
    fn a_square_mesh_has_four_corners_for_a_rim() {
        let rim = boundary(&[0, 1, 2, 0, 2, 3], 4).expect("a quad is one rim");
        assert_eq!(rim, [0, 1, 2, 3]);
    }

    /// One triangle is its own rim, and a rim that does not close is not a shape this can report.
    #[test]
    fn a_rim_has_to_be_one_closed_loop() {
        assert_eq!(boundary(&[0, 1, 2], 3), Some(vec![0, 1, 2]));
        // Two triangles that share no edge are two rims, and the walk finds only one of them.
        assert_eq!(boundary(&[0, 1, 2, 3, 4, 5], 6), None);
        // Two rims that meet at one vertex give that vertex two edges out, so the shape is not a loop.
        assert_eq!(boundary(&[0, 1, 2, 2, 3, 4], 5), None);
        // An index the mesh does not have is a mesh this cannot walk.
        assert_eq!(boundary(&[0, 1, 2], 2), None);
    }
}
