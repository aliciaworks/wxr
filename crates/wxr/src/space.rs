//! Where poses are measured from, and what a pose is.

use glam::{Affine3A, Mat4, Quat, Vec3};

/// Which reference space a pose is expressed in.
///
/// These are WebXR's, and they are a ladder from least to most information: a viewer space is the head
/// alone and moves with it, and each step after it says more about the room the user is standing in. A
/// runtime that cannot offer one of the later ones is not broken - it is a runtime with no idea where the
/// floor is, which is a thing to know rather than to fail on.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum SpaceKind {
    /// The head, at the moment it is asked. There is no world in it, only a direction and a height.
    Viewer,
    /// An origin fixed where the session began, with no claim about where the floor is.
    Local,
    /// The floor plane where the session began, with no bounds.
    LocalFloor,
    /// The floor plane and the boundary the user walked in to define it.
    BoundedFloor,
    /// An origin that moves as the runtime learns where the user is. For a walk of no fixed room.
    Unbounded,
}

impl SpaceKind {
    /// Whether this space knows where the floor is, which is what a room-scale scene has to stand on.
    pub fn has_floor(self) -> bool {
        matches!(self, Self::LocalFloor | Self::BoundedFloor)
    }
}

/// A place and an orientation.
///
/// Orientation is a quaternion and not the three numbers a runtime may report, because a runtime that
/// reports euler angles reports them in an order it does not always say - and a rig that is a quarter turn
/// out is a rig that took a day to find.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pose {
    pub position: Vec3,
    pub orientation: Quat,
}

impl Default for Pose {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Pose {
    pub const IDENTITY: Self = Self {
        position: Vec3::ZERO,
        orientation: Quat::IDENTITY,
    };

    /// The transform this pose is, for composing with geometry.
    pub fn transform(self) -> Affine3A {
        Affine3A::from_rotation_translation(self.orientation, self.position)
    }

    /// This pose, then `other`: the result is expressed in the space this one is.
    ///
    /// The order matters and is the same everywhere: a hand held relative to a shoulder, where the shoulder
    /// is relative to the floor, is `shoulder.then(hand)`.
    pub fn then(self, other: Pose) -> Pose {
        Pose {
            position: self.position + self.orientation * other.position,
            orientation: (self.orientation * other.orientation).normalize(),
        }
    }

    /// This pose seen from `from`: what it is relative to a space it was expressed in.
    pub fn relative_to(self, from: Pose) -> Pose {
        let inverse = from.orientation.inverse();
        Pose {
            position: inverse * (self.position - from.position),
            orientation: (inverse * self.orientation).normalize(),
        }
    }

    /// A pose out of a column-major 4x4, which is how a runtime that fills an array rather than answering one
    /// question at a time reports one.
    ///
    /// WebXR's `fillPoses` is the shape this is for: a browser hands back sixteen floats per pose in an array
    /// the caller owns, and there is no transform object in it anywhere - so a backend that reads a skeleton
    /// that way has a matrix and wants a place and an orientation. The layout is `glam`'s own, which is the
    /// specification's too: the three basis vectors are the first three columns and the translation is the
    /// fourth.
    ///
    /// `None` when the slice is not a matrix or the matrix is not a rigid transform, which is what an array
    /// slot a runtime could not fill looks like - a pose is a place and a rotation, and a scale of zero is
    /// neither. `fillPoses` says in its answer which slots were filled, so a caller that believed it never
    /// has a slot to ask about; this is the second line of defence for the one that did not.
    pub fn from_matrix(matrix: &[f32]) -> Option<Self> {
        if matrix.len() < 16 {
            return None;
        }
        let mut columns = [0.0f32; 16];
        columns.copy_from_slice(&matrix[..16]);
        let matrix = Mat4::from_cols_array(&columns);
        let (scale, orientation, position) = matrix.to_scale_rotation_translation();
        // A pose is a place and a rotation and nothing else, so a matrix with a scale in it is a matrix from
        // some other program - a scene node with a non-uniform scale is the usual way one arrives. The
        // rotation part of it is still a rotation, which is what makes accepting it tempting and wrong: the
        // joints would land at the right angles and the wrong distances, which is a hand in the wrong place
        // rather than a hand that failed to read. The tolerance is for the float error a runtime's own
        // multiplication leaves behind, not for a transform that means it.
        if !scale.abs_diff_eq(Vec3::ONE, 1e-3) {
            return None;
        }
        if !orientation.is_finite() || !position.is_finite() {
            return None;
        }
        let orientation = orientation.normalize();
        // Three unit axes are not a rotation: a skew keeps every column the length it was, and the
        // decomposition answers with a quaternion that is the nearest rotation rather than the one in the
        // matrix. Put that rotation back together and compare the whole thing - which also checks the last
        // row, the one a decomposition of the basis never looks at.
        let rigid = Mat4::from_scale_rotation_translation(Vec3::ONE, orientation, position);
        if !matrix.abs_diff_eq(rigid, 1e-3) {
            return None;
        }
        Some(Self {
            position,
            orientation,
        })
    }
}

/// A reference space the session has given out.
///
/// It is a handle with a kind on it rather than a transform, because a runtime is free to move the space
/// underneath - an unbounded one does - and a caller that cached a transform would be holding a stale one.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ReferenceSpace {
    pub kind: SpaceKind,
    id: u32,
}

impl ReferenceSpace {
    /// A space a backend has handed out, named by whatever it uses to tell them apart.
    pub fn new(kind: SpaceKind, id: u32) -> Self {
        Self { kind, id }
    }

    /// The backend's own name for it.
    pub fn id(self) -> u32 {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn composition_is_parent_then_child() {
        // A child one metre in front of a parent that is itself one metre up: the child is up and in front.
        let parent = Pose {
            position: Vec3::Y,
            orientation: Quat::IDENTITY,
        };
        let child = Pose {
            position: Vec3::Z,
            orientation: Quat::IDENTITY,
        };
        let world = parent.then(child);
        assert_eq!(world.position, Vec3::new(0.0, 1.0, 1.0));
    }

    #[test]
    fn a_quarter_turn_carries_the_child_with_it() {
        let parent = Pose {
            position: Vec3::ZERO,
            orientation: Quat::from_rotation_y(FRAC_PI_2),
        };
        let world = parent.then(Pose {
            position: Vec3::Z,
            orientation: Quat::IDENTITY,
        });
        // Facing has turned a forward child into a sideways one.
        assert!(
            world.position.abs_diff_eq(Vec3::X, 1e-5),
            "{:?}",
            world.position
        );
    }

    #[test]
    fn relative_to_undoes_then() {
        let parent = Pose {
            position: Vec3::new(1.0, 2.0, 3.0),
            orientation: Quat::from_rotation_x(0.4),
        };
        let child = Pose {
            position: Vec3::new(-0.5, 0.25, 1.0),
            orientation: Quat::from_rotation_y(0.7),
        };
        let round_trip = parent.then(child).relative_to(parent);
        assert!(round_trip.position.abs_diff_eq(child.position, 1e-4));
        assert!(round_trip.orientation.abs_diff_eq(child.orientation, 1e-4));
    }

    #[test]
    fn a_matrix_a_runtime_filled_is_a_pose_again() {
        let placed = Pose {
            position: Vec3::new(0.1, 1.55, -0.3),
            orientation: Quat::from_rotation_y(FRAC_PI_2),
        };
        let filled = Mat4::from(placed.transform()).to_cols_array();
        let back = Pose::from_matrix(&filled).expect("a rigid matrix is a pose");
        assert!(
            back.position.abs_diff_eq(placed.position, 1e-5),
            "{:?}",
            back
        );
        assert!(
            back.orientation.abs_diff_eq(placed.orientation, 1e-5),
            "{:?}",
            back
        );
    }

    #[test]
    fn the_translation_is_the_last_column_and_not_the_last_row() {
        // The layout spelled out rather than derived, because deriving it is the thing under test. A reader
        // that took a `fillPoses` array as row-major would find the height in the rotation instead of in the
        // translation, and hand back every joint of a hand at the origin with a strange way of turning.
        let filled = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.1, 1.55, -0.3, 1.0,
        ];
        let pose = Pose::from_matrix(&filled).expect("an identity rotation is a pose");
        assert_eq!(pose.position, Vec3::new(0.1, 1.55, -0.3));
        assert!(pose.orientation.abs_diff_eq(Quat::IDENTITY, 1e-6));
    }

    #[test]
    fn a_matrix_that_is_not_a_rigid_transform_is_not_a_pose() {
        // Which is what an array slot a runtime left alone looks like, and the reason `from_matrix` is
        // `Option` rather than a pose that is quietly somewhere else.
        assert!(Pose::from_matrix(&[]).is_none());
        let mut not_a_transform = [1.0f32; 16];
        not_a_transform[5] = f32::NAN;
        assert!(Pose::from_matrix(&not_a_transform).is_none());
        assert!(Pose::from_matrix(&[0.0; 16]).is_none());

        // A scale is a transform and not a pose: the rotation survives the decomposition, so a matrix with
        // one in it is accepted by a reader that only checks the rotation - and every joint it fills in is
        // then at the right angle and the wrong distance. An array a runtime left alone is zero, and a scene
        // node handed to `fillPoses` by mistake is the other way this happens.
        let mut scaled = Mat4::from(
            Pose {
                position: Vec3::new(0.1, 1.55, -0.3),
                orientation: Quat::from_rotation_y(FRAC_PI_2),
            }
            .transform(),
        )
        .to_cols_array();
        for axis in 0..3 {
            scaled[axis * 4] *= 2.0;
            scaled[axis * 4 + 1] *= 2.0;
            scaled[axis * 4 + 2] *= 2.0;
        }
        assert!(
            Pose::from_matrix(&scaled).is_none(),
            "a uniform scale is not a rigid transform"
        );

        // A reflection keeps the scale at one per axis in magnitude and turns the basis inside out, so the
        // quaternion that comes back is a rotation that is not the one the matrix says.
        let mut reflected = Mat4::IDENTITY.to_cols_array();
        reflected[0] = -1.0;
        assert!(
            Pose::from_matrix(&reflected).is_none(),
            "a mirror is not a pose"
        );

        // And a skew is the case three lengths cannot see: every column is still a unit vector, and none of
        // them is at a right angle to the others. A check on the decomposition's scale calls this one a pose,
        // then hands back a rotation the matrix does not describe.
        let mut skewed = Mat4::IDENTITY.to_cols_array();
        skewed[4] = 0.5;
        skewed[5] = 3.0f32.sqrt() / 2.0;
        assert!(
            Pose::from_matrix(&skewed).is_none(),
            "three unit axes that are not orthogonal are not a rotation"
        );
    }
}
