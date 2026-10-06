//! What the user is holding.
//!
//! A controller is a pose and a set of buttons, and a hand is a pose and a set of joints. What they have in
//! common - and what a core can say without knowing which it is - is *where it is* and *what it is pointed
//! at*, which is why a source is a pair of poses rather than a device.
//!
//! Buttons are the other half, and they *do* differ: OpenXR has action sets and bindings, WebXR has a
//! gamepad with an `xr-standard` button order, RealityKit has gestures. What is left after the naming is
//! subtracted is small - a main button, a grip, a stick, and how far a trigger is pulled - so that is what
//! this says, and nothing more. A game that needs the rest needs the platform, and the platform is one
//! `cfg` away.

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

/// The buttons every one of the three has, under whatever name.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Buttons {
    /// The main one: `select` in OpenXR, the trigger of an `xr-standard` gamepad, a tap on a phone.
    pub select: bool,
    /// The grip: a squeeze in OpenXR, the second gamepad button, a grip gesture.
    pub squeeze: bool,
    /// A menu or system button. Not a game's to bind on every platform - a system button may belong to the
    /// system - and worth reporting when it exists rather than pretending it does not.
    pub menu: bool,
}

/// How far the analogue things are pushed.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Axes {
    /// How far the trigger is pulled, `0..=1`.
    pub trigger: f32,
    /// The thumbstick or touchpad, `-1..=1`, with `y` up.
    pub thumbstick: glam::Vec2,
}

/// One thing the user is holding, as far as this frame knows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct InputSource {
    pub handedness: Handedness,
    /// In whichever reference space the sources were asked for.
    ///
    /// Two poses and not one, because a controller is one thing with two places on it: a grip is where it is
    /// held and an aim is where it points, and they differ on purpose - a sword and a laser pointer are the
    /// same grip and different aims. WebXR models it the same way, with a grip space and a target ray space,
    /// which is where this shape comes from.
    pub grip: Pose,
    pub aim: Pose,
    /// Whether the runtime is tracking it. An untracked controller is still a controller, and a game that
    /// forgets that teleports the player's hand to the origin.
    pub tracked: bool,
    pub buttons: Buttons,
    pub axes: Axes,
}
