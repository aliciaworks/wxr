//! The room the session is in, as triangles.
//!
//! A plane is a surface the runtime recognised; a mesh is the surface itself - the triangles, and what each
//! patch of them is. The two are the same question asked at two resolutions, which is why they are two methods
//! rather than one: a runtime that can only box things up answers `planes`, one that can trace them answers
//! both, and a game that wants to put something *on* the wall wants the triangles.
//!
//! The vocabulary is the Community Group's meshing draft - `XRMesh`, its `vertices`, `indices`, `meshSpace`
//! and `semanticLabel` - because it is the only one of the three that says this in a specification rather
//! than in a vendor's terms: OpenXR reaches it through `XR_FB_triangle_mesh` and its siblings, ARKit through
//! scene reconstruction, and both of those are the same triangle soup underneath.

use std::time::Duration;

use glam::Vec3;

use crate::space::Pose;

/// What a piece of a mesh is, which is WebXR's `semanticLabel`.
///
/// The *intersection* of the labels the three put on geometry, and not the union: ARKit's scene
/// reconstruction has a name for each of these, and the meshing draft's list is the same list. `None` is a
/// runtime that traced the triangles and did not say what they are, which is a real answer - a room to
/// collide with does not need to know that one wall is a cabinet.
///
/// A backend with a finer label than this - ARKit has no more, but a vendor's could - says the nearest of
/// these rather than widening the core: a word only one platform has a name for is the platform's, and this
/// core is the vocabulary all three share.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MeshKind {
    Wall,
    Floor,
    Ceiling,
    Table,
    Seat,
    Door,
    Window,
    Cabinet,
    Bed,
    Plant,
    Stairs,
    Tv,
    HomeAppliance,
    #[default]
    None,
}

/// A piece of the room the runtime has traced, which is WebXR's `XRMesh`.
///
/// One mesh is one *geometry*: ARKit hands over an anchor with several of them - a table and the wall behind
/// it are one anchor and two geometries - and each becomes a mesh here, because what a caller does with the
/// triangles is per-patch and not per-anchor. The `id` is therefore the geometry's, and a runtime that has no
/// geometry list uses whatever it has that is stable.
///
/// Vertices are in the mesh's own space, which is what [`Mesh::pose`] places - the draft's `meshSpace` read as
/// a pose, the same way [`crate::Plane::pose`] reads a plane space. Nothing here is normals: a triangle has
/// one, and a runtime that reports them separately is giving a caller something it can compute.
#[derive(Clone, PartialEq, Debug)]
pub struct Mesh {
    /// The backend's own identity for it, stable while the mesh is tracked - the same kind of name
    /// [`crate::InputId`] and [`crate::Plane::id`] are, and for the same reason: a mesh compared by value is
    /// two vertex lists compared.
    pub id: u32,
    /// Where the vertices are, in whichever reference space the meshes were asked for.
    pub pose: Pose,
    /// The corners, in the mesh's own space, in metres.
    pub vertices: Vec<Vec3>,
    /// Triangles into [`Mesh::vertices`], three at a time.
    pub indices: Vec<u32>,
    /// What the runtime says this patch of room is, when it says.
    pub kind: MeshKind,
    /// When it last changed, on the same clock as the frames - the draft's `lastChangedTime`. A room that
    /// has not moved has a mesh that has not changed, and a caller that rebuilds its collision on this is a
    /// caller that is not rebuilding it every frame.
    pub last_changed: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mesh_with_no_label_is_still_a_mesh() {
        let mesh = Mesh {
            id: 0,
            pose: Pose::IDENTITY,
            vertices: vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            indices: vec![0, 1, 2],
            kind: MeshKind::default(),
            last_changed: Duration::ZERO,
        };
        assert_eq!(mesh.kind, MeshKind::None);
        // Three corners and one triangle: the shape a mesh has to have to be worth anything.
        assert_eq!(mesh.indices.len() % 3, 0);
        assert!(
            mesh.indices
                .iter()
                .all(|index| (*index as usize) < mesh.vertices.len())
        );
    }
}
