//! Where a ray out of a space meets the room, cast against the shape ARKit traced of it.
//!
//! ARKit's C surface has no raycast - RealityKit has hit testing and a CompositorServices app is not in
//! RealityKit - so the room itself is the geometry: scene reconstruction's triangles and, for a device that
//! traced fewer of them, the plane anchors' rectangles. Both are already read by [`super::mesh`] and
//! [`super::plane`] for `meshes` and `planes`; this asks them a different question, so the two reads are the
//! same read and the answer is the room as it was last traced rather than as it is at the instant of the call.
//! That is one frame of staleness on a room that changes when somebody moves the furniture, and it is worth
//! writing down because a caller placing content on a hit is placing it on the room as reconstructed.
//!
//! **The normal is the placement.** WebXR's `XRHitTestResult` pose puts the surface's outward normal on +Y,
//! and a caller that puts a picture on a wall reads that axis. A triangle's winding is the runtime's and this
//! does not know which way it goes, so the front is taken to be the side the ray came from - the same side a
//! plane's +Y already faces - and a ray that lands on a back-facing triangle gets the normal turned toward it
//! rather than an upside-down placement.

use wxr::glam::{Affine3A, Quat, Vec3};

use super::ArKit;

impl ArKit {
    /// Whether a ray can be cast at all: one of the two shapes of the room came up.
    pub fn has_hit_test(&self) -> bool {
        self.has_meshes() || self.has_planes()
    }

    /// Whether a cast could report any of `kinds`.
    ///
    /// The two shapes here are the plane rectangles and the mesh triangles; feature points are a third kind
    /// the framework does not expose at all. A request that asks only for shapes this session never traced has
    /// nothing to answer with, so the source is refused rather than made; a request that also asks for one of
    /// the shapes here is answered with that one and the absent kinds simply never appear.
    pub fn has_hit_test_for(&self, kinds: wxr::HitTestKinds) -> bool {
        // Nothing asked for is nothing to answer, which is a legitimate source that reports no hits.
        if kinds.is_empty() {
            return true;
        }
        (kinds.contains(wxr::HitTestKinds::PLANES) && self.has_planes())
            || (kinds.contains(wxr::HitTestKinds::MESHES) && self.has_meshes())
    }

    /// Where `ray` meets the room, or `None` when it meets nothing.
    ///
    /// `ray` is in the session's own origin, with a direction of length one - the shape
    /// [`wxr::Session::hit_test_source`] validates it into. The nearest surface wins across the shapes asked
    /// for, because a plane is a rectangle ARKit recognised on the room a triangle patch was traced from.
    ///
    /// `kinds` narrows which shapes are read: a source asking only for planes does not pay for the triangle
    /// walk, and one asking only for meshes does not copy the plane anchors.
    pub fn cast(&self, ray: &wxr::Ray, kinds: wxr::HitTestKinds) -> Option<wxr::Pose> {
        // Only the shapes asked for are read: a read here copies the room, and a source that said it wants
        // planes should not pay for the triangles behind them.
        let meshes = wants(kinds, wxr::HitTestKinds::MESHES).then(|| self.meshes());
        let planes = wants(kinds, wxr::HitTestKinds::PLANES).then(|| self.planes());
        let contact = nearest(
            meshes.as_deref().unwrap_or_default(),
            planes.as_deref().unwrap_or_default(),
            ray,
        )?;
        Some(wxr::Pose {
            position: contact.point,
            // The shortest rotation from +Y to the normal is the whole of the placement: the roll around the
            // normal is not something WebXR says anything about, and a caller that cares sets its own.
            orientation: Quat::from_rotation_arc(Vec3::Y, contact.normal),
        })
    }
}

/// Whether a source that asked for `kinds` wants `kind` read, which is what keeps a narrowed cast cheap.
fn wants(kinds: wxr::HitTestKinds, kind: wxr::HitTestKinds) -> bool {
    kinds.contains(kind)
}

/// Where a ray met the room: an outward normal and the point on it, both in the session's origin.
struct Contact {
    point: Vec3,
    normal: Vec3,
    /// How far along the ray, which is what "nearest" is measured by.
    distance: f32,
}

/// The nearest contact across the room's triangles and its rectangles, if there is one.
fn nearest(meshes: &[wxr::Mesh], planes: &[wxr::Plane], ray: &wxr::Ray) -> Option<Contact> {
    let mut nearest: Option<Contact> = None;
    for mesh in meshes {
        let place: Affine3A = mesh.pose.transform();
        // The mesh's own space, which is where its vertices are: the ray is taken to it rather than every
        // vertex being taken out of it.
        let to_mesh = place.inverse();
        let origin = to_mesh.transform_point3(ray.origin);
        let direction = to_mesh.transform_vector3(ray.direction);
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let Some(corners) = corners(mesh, triangle) else {
                continue;
            };
            let Some((distance, normal)) = intersect_triangle(origin, direction, corners) else {
                continue;
            };
            let point = place.transform_point3(origin + direction * distance);
            let normal = facing(place.transform_vector3(normal).normalize(), ray);
            consider(
                &mut nearest,
                Contact {
                    point,
                    normal,
                    distance,
                },
            );
        }
    }
    for plane in planes {
        let Some(contact) = intersect_plane(plane, ray) else {
            continue;
        };
        consider(&mut nearest, contact);
    }
    nearest
}

/// Keep `contact` if it is the nearest one yet.
fn consider(nearest: &mut Option<Contact>, contact: Contact) {
    if nearest
        .as_ref()
        .is_none_or(|best| contact.distance < best.distance)
    {
        *nearest = Some(contact);
    }
}

/// The three corners a triangle of `mesh` names, or `None` when one of them is not a vertex.
fn corners(mesh: &wxr::Mesh, triangle: &[u32; 3]) -> Option<[Vec3; 3]> {
    Some([
        *mesh.vertices.get(triangle[0] as usize)?,
        *mesh.vertices.get(triangle[1] as usize)?,
        *mesh.vertices.get(triangle[2] as usize)?,
    ])
}

/// Where a unit ray meets a triangle: how far, and the triangle's normal made unit.
///
/// A back face is a hit like any other - the room is a closed surface and a ray from inside it leaves through
/// the front of something - so the winding is not tested here, and [`facing`] is what decides which side is
/// outward.
fn intersect_triangle(origin: Vec3, direction: Vec3, [a, b, c]: [Vec3; 3]) -> Option<(f32, Vec3)> {
    let normal = (b - a).cross(c - a);
    let denominator = normal.dot(direction);
    // The plane the triangle is in, seen edge-on: there is no one point of it the ray meets. A triangle with
    // no area is caught by this too, because its normal is zero.
    if denominator.abs() < f32::EPSILON {
        return None;
    }
    let distance = normal.dot(a - origin) / denominator;
    if !distance.is_finite() || distance < 0.0 {
        return None;
    }
    // Inside the triangle, by the same three edges: a point on the plane is inside when it is on the inner
    // side of all three, and the plane's own normal is what says which side that is.
    let point = origin + direction * distance;
    if (b - a).cross(point - a).dot(normal) < 0.0
        || (c - b).cross(point - b).dot(normal) < 0.0
        || (a - c).cross(point - c).dot(normal) < 0.0
    {
        return None;
    }
    Some((distance, normal.normalize()))
}

/// Where a unit ray meets a plane anchor's rectangle, with the plane's own +Y as the normal.
fn intersect_plane(plane: &wxr::Plane, ray: &wxr::Ray) -> Option<Contact> {
    let place = plane.pose.transform();
    let to_plane = place.inverse();
    let origin = to_plane.transform_point3(ray.origin);
    let direction = to_plane.transform_vector3(ray.direction);
    // The plane is `y = 0` in its own space, so a ray that does not cross that is not a hit - and a ray lying
    // in it meets infinitely many points rather than one.
    if direction.y.abs() < f32::EPSILON {
        return None;
    }
    let distance = -origin.y / direction.y;
    if !distance.is_finite() || distance < 0.0 {
        return None;
    }
    let point = origin + direction * distance;
    // The extent is a box and the pose is its centre, so both halves are half the extent - and the plane's
    // own axes are X and Z, which is what makes the extent a `Vec2`.
    if point.x.abs() > plane.extent.x * 0.5 || point.z.abs() > plane.extent.y * 0.5 {
        return None;
    }
    Some(Contact {
        point: place.transform_point3(point),
        normal: facing((plane.pose.orientation * Vec3::Y).normalize(), ray),
        distance,
    })
}

/// `normal` turned to the side the ray came from, which is the side a placement faces.
fn facing(normal: Vec3, ray: &wxr::Ray) -> Vec3 {
    if normal.dot(ray.direction) > 0.0 {
        -normal
    } else {
        normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wxr::glam::{Vec2, Vec3};

    /// A mesh whose triangles are in its own space, placed by `pose`.
    fn mesh(pose: wxr::Pose, vertices: Vec<Vec3>, indices: Vec<u32>) -> wxr::Mesh {
        wxr::Mesh {
            id: 0,
            pose,
            vertices,
            indices,
            kind: wxr::SurfaceKind::None,
            last_changed: std::time::Duration::ZERO,
        }
    }

    /// A rectangle of `extent` metres centred on `pose`, with the normal on its +Y.
    fn plane(pose: wxr::Pose, extent: Vec2) -> wxr::Plane {
        wxr::Plane {
            id: 0,
            pose,
            extent,
            orientation: wxr::PlaneOrientation::Horizontal,
            kind: wxr::SurfaceKind::None,
            last_changed: None,
        }
    }

    /// One square metre of wall across the XY plane at `z`.
    fn wall(z: f32) -> wxr::Mesh {
        mesh(
            wxr::Pose {
                position: Vec3::new(0.0, 0.0, z),
                ..wxr::Pose::IDENTITY
            },
            vec![
                Vec3::new(-1.0, -1.0, 0.0),
                Vec3::new(1.0, -1.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(-1.0, 1.0, 0.0),
            ],
            vec![0, 1, 2, 0, 2, 3],
        )
    }

    /// The pose a contact is answered with, which is what a caller reads.
    fn placement(contact: &Contact) -> wxr::Pose {
        wxr::Pose {
            position: contact.point,
            orientation: Quat::from_rotation_arc(Vec3::Y, contact.normal),
        }
    }

    #[test]
    fn a_ray_through_a_triangle_lands_on_it_and_the_pose_looks_back_down_the_ray() {
        let ray = wxr::Ray {
            origin: Vec3::new(0.2, -0.3, 1.0),
            direction: Vec3::NEG_Z,
        };
        let contact = nearest(&[wall(0.0)], &[], &ray).expect("the ray is aimed at the wall");
        let pose = placement(&contact);
        assert!((pose.position - Vec3::new(0.2, -0.3, 0.0)).length() < 1e-5);
        // Up on the wall is out of it, toward the side the ray came from.
        assert!((pose.orientation * Vec3::Y - Vec3::Z).length() < 1e-5);
    }

    #[test]
    fn a_ray_beside_the_wall_meets_nothing() {
        let ray = wxr::Ray {
            origin: Vec3::new(1.5, 0.0, 1.0),
            direction: Vec3::NEG_Z,
        };
        assert!(nearest(&[wall(0.0)], &[], &ray).is_none());
    }

    #[test]
    fn a_mesh_pose_is_where_its_triangles_are_and_the_normal_is_turned_toward_the_ray() {
        // The same square laid flat: rotating +90 about X puts its local Z on world -Y, so a ray from above
        // lands on what the winding would call its back.
        let flat = wall(0.0);
        let floor = mesh(
            wxr::Pose {
                position: Vec3::ZERO,
                orientation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            },
            flat.vertices,
            flat.indices,
        );
        let ray = wxr::Ray {
            origin: Vec3::new(0.0, 2.0, 0.0),
            direction: Vec3::NEG_Y,
        };
        let contact = nearest(&[floor], &[], &ray).expect("the ray is aimed at the floor");
        let pose = placement(&contact);
        assert!(pose.position.length() < 1e-5);
        // Up out of the floor, which is the side the ray came from and not the side the winding picked.
        assert!((pose.orientation * Vec3::Y - Vec3::Y).length() < 1e-5);
    }

    #[test]
    fn a_plane_is_its_rectangle_and_not_the_whole_of_its_own_plane() {
        let floor = plane(wxr::Pose::IDENTITY, Vec2::new(2.0, 2.0));
        let inside = wxr::Ray {
            origin: Vec3::new(0.5, 2.0, 0.5),
            direction: Vec3::NEG_Y,
        };
        let outside = wxr::Ray {
            origin: Vec3::new(1.5, 2.0, 0.0),
            direction: Vec3::NEG_Y,
        };
        let contact = nearest(&[], &[floor], &inside).expect("half a metre is on the floor");
        assert!((contact.point - Vec3::new(0.5, 0.0, 0.5)).length() < 1e-5);
        assert!((contact.normal - Vec3::Y).length() < 1e-5);
        assert!(nearest(&[], &[floor], &outside).is_none());
    }

    #[test]
    fn the_nearest_of_a_triangle_and_a_rectangle_is_the_one_reported() {
        let ray = wxr::Ray {
            origin: Vec3::new(0.0, 0.0, 1.0),
            direction: Vec3::NEG_Z,
        };
        // A wall at the origin and another one metre behind it: the ray crosses both, and the answer is the
        // one in front.
        let behind = plane(
            wxr::Pose {
                position: Vec3::new(0.0, 0.0, -1.0),
                // A plane's normal is its own +Y, so a wall is the floor turned upright.
                orientation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            },
            Vec2::new(4.0, 4.0),
        );
        let contact = nearest(&[wall(0.0)], &[behind], &ray).expect("the ray meets the wall");
        assert!(contact.point.length() < 1e-5);
        // The same two shapes with the rectangle now in front.
        let front = plane(
            wxr::Pose {
                position: Vec3::new(0.0, 0.0, 0.5),
                orientation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            },
            Vec2::new(4.0, 4.0),
        );
        let contact = nearest(&[wall(0.0)], &[front], &ray).expect("the ray meets the plane");
        assert!((contact.point - Vec3::new(0.0, 0.0, 0.5)).length() < 1e-5);
    }

    #[test]
    fn a_narrowed_source_reads_only_the_shapes_it_asked_for() {
        assert!(wants(wxr::HitTestKinds::ALL, wxr::HitTestKinds::MESHES));
        assert!(wants(wxr::HitTestKinds::ALL, wxr::HitTestKinds::PLANES));
        assert!(!wants(wxr::HitTestKinds::PLANES, wxr::HitTestKinds::MESHES));
        assert!(!wants(wxr::HitTestKinds::MESHES, wxr::HitTestKinds::PLANES));
        // Nothing asked for is nothing read, which is the source that can never report a hit.
        assert!(!wants(wxr::HitTestKinds::NONE, wxr::HitTestKinds::PLANES));
        // And a plane list emptied by a narrowed request is a cast that meets nothing.
        let ray = wxr::Ray {
            origin: Vec3::new(0.0, 0.0, 1.0),
            direction: Vec3::NEG_Z,
        };
        assert!(nearest(&[], &[], &ray).is_none());
    }
}
