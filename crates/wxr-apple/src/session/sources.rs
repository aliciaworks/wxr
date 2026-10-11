//! The session's input sources: where they come from, and what a hand's skeleton is once it is Rust's.
//!
//! Separate from `dispatch` for the reason the other two backends separate theirs: the `wxr::Session`
//! implementation is one block of answers, and this is the part of it about input - which is read for a
//! different question than the answers are. `dispatch` asks *what does this session say*; this asks *how is
//! a source found and how are its joints read*.
//!
//! **The two frameworks do not overlap and neither is optional.** Controllers are GameController's and hands
//! ARKit's, and a session can have either without the other - a Mac with a pad and no hands to track, or a
//! headset with hands and no pad plugged in. So each half returns early on its own missing framework and
//! neither returns early for the other's.
//!
//! **A hand's `InputId` is its index in the order `ArKit::hands` reports, which is left then right.** That
//! is the same order every frame, because the query fills one left anchor and one right anchor and reports
//! them in that order - so an id means the same hand across frames, which is what an event referring to a
//! source has to mean. Controllers are past [`CONTROLLER_ID_BASE`] so the two cannot collide.

use super::*;
use crate::gamecontroller::CONTROLLER_ID_BASE;

impl AppleSession {
    /// The set of things the user is holding, as this frame knows them.
    ///
    /// Hands are picked up and put down, and that is the set of inputs changing - one event however many
    /// hands it is about, which is the shape WebXR gives `inputsourceschange` too.
    pub(super) fn inputs_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::InputSource>,
    ) -> Result<(), wxr::Error> {
        // Controllers are GameController's and hands are ARKit's, so this comes first and on its own: a
        // session whose ARKit never came up still has controllers, and the early return below is about one
        // framework rather than about both.
        if self.controllers.read(out) {
            self.inputs_changed = true;
        }
        let Some(arkit) = &self.arkit else {
            return Ok(());
        };
        // The same space the views are expressed in, for the same reason.
        let origin = self.space_origin(space);

        let hands = arkit.hands();
        if hands.len() != self.hands_seen {
            self.hands_seen = hands.len();
            self.inputs_changed = true;
        }

        for (index, (handedness, transform, tracked)) in hands.into_iter().enumerate() {
            let pose = wxr_render::pose_from_transform(origin * transform);
            out.push(wxr::InputSource {
                id: wxr::InputId::new(index as u32),
                handedness,
                // A palm ray is a tracked pointer like any other; the C surface has no gaze or screen ray to
                // make.
                target_ray_mode: wxr::TargetRayMode::TrackedPointer,
                // A skeleton, now that the C surface's joint transforms are read - see [`skeleton`]. This is
                // the question `Session::hand` wants asked first, and it is about whether there is a
                // skeleton to ask for rather than a promise that every joint is there this frame.
                hand: true,
                // And nothing to buzz, because a hand does not: the controllers that can are reported beside
                // it, from GameController rather than ARKit - see `crate::gamecontroller`.
                haptics: false,
                // A hand is a place and an orientation, and the pose's own orientation is the palm's
                // direction: a game that wants a ray takes its -Z, which is what every ray in this workspace
                // is. The aim is the same pose as the grip - there is no second point on a palm for ARKit to
                // report.
                grip: pose,
                aim: pose,
                tracked,
                // And no buttons at all, which is not a gap: a hand has none. The core's buttons are the
                // intersection of the three platforms, and this is the platform where the intersection is
                // empty - so there are no press events here either, because there is nothing to press.
                buttons: wxr::Buttons::default(),
                axes: wxr::Axes::default(),
            });
        }
        Ok(())
    }

    /// One hand's skeleton, in `space`.
    ///
    /// A source that is not one of the two hands - a controller, whose ids begin at [`CONTROLLER_ID_BASE`] -
    /// is an empty hand rather than an error: the core asks before it asks, and a game that asked anyway is
    /// a game whose question was already answered by [`wxr::InputSource::hand`].
    pub(super) fn hand_impl(
        &mut self,
        source: wxr::InputId,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Hand,
    ) -> Result<(), wxr::Error> {
        out.clear();
        let Some(arkit) = &self.arkit else {
            return Ok(());
        };
        // A controller's id is past the offset its own module starts at, and a hand's is its index in the
        // order `ArKit::hands` reports - left, then right. Written as the offset rather than as a bare
        // `< 2`, so that a controller id is refused for the reason it is actually refused: it is not one of
        // the two hand slots, and the number that says so is the same one `gamecontroller` numbers from.
        let index = source.get();
        let Some(handedness) = (index < CONTROLLER_ID_BASE)
            .then_some(index as usize)
            .and_then(|index| HANDEDNESS.get(index))
        else {
            return Ok(());
        };
        // The same space the palm beside it was brought to, so that a joint and the hand it is on land in
        // the same place - which is the property that has to hold, or a drawn hand comes apart at the wrist.
        let origin = self.space_origin(space);
        if let Some((_tracked, hand)) = arkit.hand(*handedness, origin) {
            *out = hand;
        }
        Ok(())
    }
}

/// Which hand each of the first input ids is, in the order `ArKit::hands` reports them.
///
/// Two, because that is the number of hands a person has: `ArKit::hands` fills one left anchor and one right
/// anchor and reports them in that order, so an id is the same hand every frame.
const HANDEDNESS: [wxr::Handedness; 2] = [wxr::Handedness::Left, wxr::Handedness::Right];
