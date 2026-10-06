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
//!
//! What a source *does* is the other half, and it is not a snapshot: a press begins, ends, and counts as a
//! selection, and those are events on the session rather than fields on a source. WebXR draws the line there,
//! and it is the right one - a frame tells you where the hands are, and a session tells you what they did.

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

/// What a source is aimed by, which is WebXR's `XRTargetRayMode`.
///
/// It says how the ray a source points with was made, not what the source is - a controller and an articulated
/// hand are both tracked pointers - and it is what a game asks before it draws one: a gaze ray comes from the
/// head and has no place of its own, a screen ray comes from a touch, and a transient ray is a tap that is over
/// by the time it is read.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TargetRayMode {
    /// Tracked in space and held or worn: a controller, a tracked stylus, a hand.
    #[default]
    TrackedPointer,
    /// Pointed by looking, with the ray at the head.
    Gaze,
    /// A touch on a flat screen, in a session that is not immersive. The ray is where the finger is.
    Screen,
    /// A tap that exists for the moment it is read, with nothing left to track afterwards.
    TransientPointer,
}

/// Which source an event is about.
///
/// WebXR names a source by the object it is, which a Rust value cannot: an event arrives with a source in it,
/// and two sources compared by value are two poses compared. So the core carries the backend's own identity
/// instead - stable for as long as the source exists, and meaningful only to the backend that handed it out.
/// It is the same kind of concession as [`crate::State::Connecting`]: a fact a synchronous API has to spell out
/// and a promise does not.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct InputId(u32);

impl InputId {
    /// A backend's own name for a source.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// The number the backend made it from.
    pub fn get(self) -> u32 {
        self.0
    }
}

/// A joint of a hand, which is WebXR's `XRHandJoint` and nothing but its names.
///
/// Twenty-five of them: the wrist, and then five places along each finger. The thumb is the one that reads
/// differently, because it has no intermediate phalanx and a metacarpal of its own. OpenXR has the same list
/// with a palm in front of it, and a palm is not a joint of anything - so a backend that has one leaves it out
/// rather than the core carrying a name only one platform says.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum HandJoint {
    Wrist,
    ThumbMetacarpal,
    ThumbPhalanxProximal,
    ThumbPhalanxDistal,
    ThumbTip,
    IndexFingerMetacarpal,
    IndexFingerPhalanxProximal,
    IndexFingerPhalanxIntermediate,
    IndexFingerPhalanxDistal,
    IndexFingerTip,
    MiddleFingerMetacarpal,
    MiddleFingerPhalanxProximal,
    MiddleFingerPhalanxIntermediate,
    MiddleFingerPhalanxDistal,
    MiddleFingerTip,
    RingFingerMetacarpal,
    RingFingerPhalanxProximal,
    RingFingerPhalanxIntermediate,
    RingFingerPhalanxDistal,
    RingFingerTip,
    PinkyFingerMetacarpal,
    PinkyFingerPhalanxProximal,
    PinkyFingerPhalanxIntermediate,
    PinkyFingerPhalanxDistal,
    PinkyFingerTip,
}

impl HandJoint {
    /// How many there are, which is how many a [`Hand`] holds.
    pub const COUNT: usize = 25;

    /// Every joint, in WebXR's order, which is the order a [`Hand`] is in.
    pub const ALL: [Self; Self::COUNT] = [
        Self::Wrist,
        Self::ThumbMetacarpal,
        Self::ThumbPhalanxProximal,
        Self::ThumbPhalanxDistal,
        Self::ThumbTip,
        Self::IndexFingerMetacarpal,
        Self::IndexFingerPhalanxProximal,
        Self::IndexFingerPhalanxIntermediate,
        Self::IndexFingerPhalanxDistal,
        Self::IndexFingerTip,
        Self::MiddleFingerMetacarpal,
        Self::MiddleFingerPhalanxProximal,
        Self::MiddleFingerPhalanxIntermediate,
        Self::MiddleFingerPhalanxDistal,
        Self::MiddleFingerTip,
        Self::RingFingerMetacarpal,
        Self::RingFingerPhalanxProximal,
        Self::RingFingerPhalanxIntermediate,
        Self::RingFingerPhalanxDistal,
        Self::RingFingerTip,
        Self::PinkyFingerMetacarpal,
        Self::PinkyFingerPhalanxProximal,
        Self::PinkyFingerPhalanxIntermediate,
        Self::PinkyFingerPhalanxDistal,
        Self::PinkyFingerTip,
    ];

    /// Which joint of a [`Hand`] this is.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// One joint of a hand at one frame.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Joint {
    /// Where it is, in whichever reference space the hand was asked for.
    pub pose: Pose,
    /// How thick the finger is there, in metres. OpenXR and WebXR both report it, and a hand drawn without it
    /// is a stick figure.
    pub radius: f32,
}

/// A hand's skeleton, which is WebXR's `XRHand` read as a pose per joint.
///
/// A fixed array rather than a list, because the joints are a fixed set and a caller indexing by joint should
/// not have to ask whether the list is complete: `None` is a joint that is not tracked, which is a hand half out
/// of view and not an error.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Hand {
    joints: [Option<Joint>; HandJoint::COUNT],
}

impl Default for Hand {
    fn default() -> Self {
        Self {
            joints: [None; HandJoint::COUNT],
        }
    }
}

impl Hand {
    /// Where a joint is, if it is tracked this frame.
    pub fn joint(&self, joint: HandJoint) -> Option<Joint> {
        self.joints[joint.index()]
    }

    /// The joints, for a backend to fill. Emptying it first is the backend's business.
    pub fn joints_mut(&mut self) -> &mut [Option<Joint>; HandJoint::COUNT] {
        &mut self.joints
    }

    /// Forget every joint, which is what a frame with no hand in view looks like.
    pub fn clear(&mut self) {
        self.joints = [None; HandJoint::COUNT];
    }

    /// Whether any of it is tracked, which is what a hand that is there at all looks like.
    pub fn is_tracked(&self) -> bool {
        self.joints.iter().any(Option::is_some)
    }
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
    /// The backend's own identity for this source, so an event can say which one it is about.
    pub id: InputId,
    pub handedness: Handedness,
    /// How the source is aimed, which is not what it is: a controller and a hand are both tracked pointers.
    pub target_ray_mode: TargetRayMode,
    /// Whether there is a skeleton to ask for, which is WebXR's `hand` being non-null.
    ///
    /// A controller has none, and neither does a platform whose hands are a pose and no fingers - so this is a
    /// question to ask before [`crate::Session::hand`], not a promise that the fingers are there.
    pub hand: bool,
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
