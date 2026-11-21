//! Asking where the world is, which is WebXR's hit test.

use crate::input::InputId;
use crate::space::Pose;

use glam::Vec3;

/// Where a ray starts and which way it goes, which is WebXR's `XRRay` in the half that matters.
///
/// In the coordinates of the space it is given with, and in metres - a ray is not a pose, so there is no
/// orientation to compose and no fifth number for a point at infinity. `direction` needs no unit length: what
/// reads it normalises it, and [`Ray::unit`] is that normalising done once where it can be refused.
///
/// The `matrix` an `XRRay` also has is the ray as a transform, which is what a runtime keeps internally; a
/// caller that wants it can build it, and a core that carried both would have two ways to say one thing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ray {
    /// Where it starts, in the space's own coordinates.
    pub origin: Vec3,
    /// Which way it goes, in the space's coordinates.
    pub direction: Vec3,
}

impl Ray {
    /// The same ray with a unit direction, or `None` when it does not describe one.
    ///
    /// A zero direction is not a place to look, and neither is one with a NaN or an infinity in it, or an
    /// origin that is not three finite numbers - normalising those invents an answer rather than finding
    /// one. A backend refuses such a ray at the call that took it rather than handing it to a runtime that
    /// would cast it and report a hit at a point it made up.
    pub fn unit(self) -> Option<Self> {
        if !self.origin.is_finite() {
            return None;
        }
        Some(Self {
            origin: self.origin,
            direction: self.direction.try_normalize()?,
        })
    }
}

/// Where a ray met the world, which is WebXR's `XRHitTestResult`.
///
/// A pose and, at most, whose ray asked: *what* was met is the runtime's business, and a scene that wants to
/// put something where the ray landed wants the pose.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Hit {
    pub pose: Pose,
    /// Which of the app's own sources aimed the ray, when it was one of theirs rather than one the app
    /// described - WebXR's `XRTransientInputHitTestResult.inputSource`. `None` for a source asked for along a
    /// ray the app gave, which is every hit from [`Session::hit_test_source`](crate::Session::hit_test_source);
    /// two fingers on a touchscreen ask through one profile source and come back told apart by this.
    pub input: Option<InputId>,
}

/// A ray asking where the world is, which is WebXR's `XRHitTestSource`.
///
/// A handle and not the ray itself, because a runtime makes one asynchronously and it outlives the frame that
/// asked for it - the same shape [`crate::ReferenceSpace`] has, and for the same reason: the thing is the
/// runtime's, and what a caller holds is its name.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct HitTestSource {
    id: u32,
}

impl HitTestSource {
    /// A source a backend has handed out, named by whatever it uses to tell them apart.
    pub fn new(id: u32) -> Self {
        Self { id }
    }

    /// The backend's own name for it.
    pub fn id(self) -> u32 {
        self.id
    }
}

/// What a hit test is allowed to land on, which is WebXR's `XRHitTestTrackableType` as a set.
///
/// A plane and a mesh are the same wall at two resolutions: a runtime that recognises surfaces has rectangles,
/// and one that traces them has triangles. An app placing a picture wants the wall it actually has rather than
/// a rectangle guessed around it, and this is how it says so - WebXR's `entityTypes`, whose default is
/// `["plane"]` and which this core makes explicit instead.
///
/// The three kinds are not three qualities of answer, and a runtime need not have all of them: ARKit traces
/// meshes and recognises planes, and neither backend here exposes feature points at all. So this is a
/// *request*: a backend that has none of the kinds asked for refuses the source with
/// [`Error::Unsupported`](crate::Error::Unsupported) rather than handing back one that never answers, and a
/// backend that has some of them answers with those.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HitTestKinds(u8);

impl HitTestKinds {
    /// Nothing, which is a source that will never report a hit - a legitimate thing to ask for and a strange
    /// one to want.
    pub const NONE: Self = Self(0);
    /// The runtime's feature points, which WebXR has and which the two native backends here do not expose.
    pub const POINTS: Self = Self(1 << 0);
    /// The rectangles a runtime recognised, which is [`crate::Plane`].
    pub const PLANES: Self = Self(1 << 1);
    /// The triangles a runtime traced, which is [`crate::Mesh`].
    pub const MESHES: Self = Self(1 << 2);
    /// Everything, which is what a caller with no preference asks for and what
    /// [`HitTestKinds::default`] is.
    pub const ALL: Self = Self(Self::POINTS.0 | Self::PLANES.0 | Self::MESHES.0);

    /// Whether every kind in `other` is in this set.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether the two sets have a kind in common, which is what a source's request has to do with what a
    /// backend can actually report.
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Whether there is nothing here.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// A set with everything that is in either.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl Default for HitTestKinds {
    /// Everything, because a backend can always narrow what it reports and a caller that has not thought about
    /// the kinds is a caller that wants whatever is in front of it.
    fn default() -> Self {
        Self::ALL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kinds_are_a_set_and_all_holds_every_one() {
        assert!(HitTestKinds::ALL.contains(HitTestKinds::PLANES));
        assert!(HitTestKinds::ALL.contains(HitTestKinds::MESHES.union(HitTestKinds::POINTS)));
        assert!(!HitTestKinds::PLANES.contains(HitTestKinds::MESHES));
        assert!(HitTestKinds::PLANES.intersects(HitTestKinds::POINTS.union(HitTestKinds::PLANES)));
        assert!(!HitTestKinds::POINTS.intersects(HitTestKinds::PLANES));
        assert!(HitTestKinds::NONE.is_empty());
        assert!(!HitTestKinds::POINTS.is_empty());
        assert_eq!(HitTestKinds::default(), HitTestKinds::ALL);
    }

    #[test]
    fn a_ray_is_made_a_unit_length_and_a_direction_that_is_not_one_is_refused() {
        let ray = Ray {
            origin: Vec3::new(1.0, 2.0, 3.0),
            // Ten times too long, which is a direction all the same and comes back at length one.
            direction: Vec3::new(0.0, 0.0, -5.0),
        };
        let unit = ray.unit().expect("a direction is a direction");
        assert_eq!(unit.origin, ray.origin);
        assert!((unit.direction.length() - 1.0).abs() < 1e-6);
        assert!((unit.direction.z + 1.0).abs() < 1e-6);

        // Three ways of saying nothing about where to look, and all three are the same refusal.
        for direction in [
            Vec3::ZERO,
            Vec3::splat(f32::NAN),
            Vec3::splat(f32::INFINITY),
        ] {
            assert!(Ray { direction, ..ray }.unit().is_none(), "{direction:?}");
        }
        // And a ray that starts nowhere is refused even when it points somewhere.
        assert!(
            Ray {
                origin: Vec3::splat(f32::NAN),
                ..ray
            }
            .unit()
            .is_none()
        );
    }
}
