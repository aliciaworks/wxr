//! Asking where the world is, which is WebXR's hit test.

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
/// A pose and nothing more, because that is all WebXR gives one: *what* was met is the runtime's business, and
/// a scene that wants to put something where the ray landed wants the pose.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Hit {
    pub pose: Pose,
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

#[cfg(test)]
mod tests {
    use super::*;

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
