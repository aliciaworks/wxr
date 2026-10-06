//! Asking where the world is, which is WebXR's hit test.

use crate::space::Pose;

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
