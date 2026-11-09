//! The two transforms between ARKit's world and the space a caller asked for.
//!
//! ARKit reports the room, a hand and an anchor in the coordinate space its session started in, which is
//! `Local`; the compositor reports an eye in device space, which is the wearer's head. A reference space is
//! one or the other with an offset, so moving between them is one multiplication each way - and the two are
//! easy to swap, which is why they are here with a test rather than inline at each use.

use wxr::glam::Mat4;
use wxr::{ReferenceSpace, SpaceKind};

/// Whether a session's own list of spaces holds the one a handle names.
///
/// The id is the session's index and the kind is public on the handle, so both have to agree: an id past
/// the end is another session's, and a kind a caller changed is not the space this session made.
pub(super) fn has(spaces: &[(SpaceKind, wxr::Pose)], space: ReferenceSpace) -> bool {
    spaces
        .get(space.id() as usize)
        .is_some_and(|(kind, _)| *kind == space.kind)
}

/// `space` seen from ARKit's world: a world pose is this composed on the left.
///
/// `device_in_space` is the device's pose in `space`, and `world_from_device` is the device's pose in ARKit's
/// world. Taking the device back out of the first and putting it into the second is what turns a pose that
/// moves with the room into one that moves with the head.
///
/// No device pose this frame means no prediction: the identity stands in, which leaves `Local` exactly the
/// world and a viewer space head-locked, the same fallback the views take.
pub(super) fn space_from_world(device_in_space: Mat4, world_from_device: Option<Mat4>) -> Mat4 {
    device_in_space * world_from_device.unwrap_or(Mat4::IDENTITY).inverse()
}

/// ARKit's world seen from `space`: a pose in `space` is this composed on the left.
pub(super) fn world_from_space(device_in_space: Mat4, world_from_device: Option<Mat4>) -> Mat4 {
    world_from_device.unwrap_or(Mat4::IDENTITY) * device_in_space.inverse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wxr::glam::{Vec3, vec3};

    /// A device a metre into the room, which is enough to tell a transform that uses its pose from one that
    /// ignores it.
    fn device() -> Mat4 {
        Mat4::from_translation(vec3(0.0, 0.0, 1.0))
    }

    /// A plane to the device's right, which is where the room keeps it whether or not the head moves.
    fn plane() -> Mat4 {
        Mat4::from_translation(vec3(1.0, 0.0, 0.0))
    }

    fn position(transform: Mat4) -> Vec3 {
        transform.w_axis.truncate()
    }

    #[test]
    fn local_is_the_world_so_a_room_pose_passes_through() {
        // `Local` is ARKit's own origin, so the device's pose in it is the device's world pose and the two
        // cancel: a plane at (1, 0, 0) stays at (1, 0, 0) however far the head has walked.
        let space = space_from_world(device(), Some(device()));
        assert!(position(space * plane()).abs_diff_eq(vec3(1.0, 0.0, 0.0), 1e-6));
    }

    #[test]
    fn a_viewer_space_is_the_world_seen_from_the_head() {
        // Device space has the head at its origin, so the same plane is a metre to the right and a metre in
        // front of the wearer - which is the world move a backend that used the device's pose twice would
        // get wrong.
        let space = space_from_world(Mat4::IDENTITY, Some(device()));
        assert!(position(space * plane()).abs_diff_eq(vec3(1.0, 0.0, -1.0), 1e-6));
    }

    #[test]
    fn the_two_ways_round_undo_each_other() {
        let there = space_from_world(device(), Some(device()));
        let back = world_from_space(device(), Some(device()));
        assert!((back * there).abs_diff_eq(Mat4::IDENTITY, 1e-6));
    }

    /// A handle names a space only when both halves agree: the id is one this session made, and the kind is
    /// the kind it made of that id. Either one changed is another session's handle, or a lie about this one.
    #[test]
    fn a_space_needs_its_own_id_and_its_own_kind() {
        let spaces = [(SpaceKind::Local, wxr::Pose::IDENTITY)];
        assert!(has(&spaces, ReferenceSpace::new(SpaceKind::Local, 0)));
        assert!(
            !has(&spaces, ReferenceSpace::new(SpaceKind::Viewer, 0)),
            "the kind is public, so a handle can say one this session never made"
        );
        assert!(
            !has(&spaces, ReferenceSpace::new(SpaceKind::Local, 1)),
            "an id past the list belongs to some other session"
        );
    }
}
