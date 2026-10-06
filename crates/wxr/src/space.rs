//! Where poses are measured from, and what a pose is.

use glam::{Affine3A, Quat, Vec3};

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
}
