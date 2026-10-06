//! What the user is holding.
//!
//! A controller is a pose and a set of buttons, and a hand is a pose and a set of joints. What they have in
//! common - and what a core can say without knowing which it is - is *where it is* and *what it is pointed
//! at*, which is why a source is a pair of poses rather than a device.
//!
//! Buttons are not here yet. They are the half that differs most between the three platforms - OpenXR has
//! action sets and bindings, WebXR has gamepads and profiles, RealityKit has gestures - and a shape guessed
//! now would be a shape to break later. Poses do not differ, so they are the half that is.

use crate::space::Pose;

/// Which hand a source is, if the runtime knows.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Handedness {
    Left,
    Right,
    /// A runtime that does not say, or a source that is not a hand - a gaze cursor, a tracker.
    #[default]
    Unknown,
}

/// Which of a source's two poses is wanted.
///
/// They point in different directions on purpose: a grip is where the hand is, and an aim is where the
/// thing in it is pointed - a sword and a laser pointer are the same grip and different aims.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Grip {
    /// Where the user is holding it.
    #[default]
    Grip,
    /// Where it is pointed.
    Aim,
}

/// One thing the user is holding, as far as this frame knows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct InputSource {
    pub handedness: Handedness,
    pub grip: Grip,
    /// In whichever reference space the sources were asked for.
    pub pose: Pose,
    /// Whether the runtime is tracking it. An untracked controller is still a controller, and a game that
    /// forgets that teleports the player's hand to the origin.
    pub tracked: bool,
}
