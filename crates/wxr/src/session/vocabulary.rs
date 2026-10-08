//! The words a session is described in: what it is doing, whether it is shown, and what it says happened.
//!
//! They are separate from the traits because they are what a *caller* is written against - a game matches
//! on [`Event`] and asks for a [`SessionMode`] - and the traits are what a backend is written against. The
//! two are read by different people, and this file is the one the first of them opens.

use super::*;

/// What the session's lifecycle is: the smallest thing a synchronous API needs and a promise does not.
///
/// WebXR has no session state machine. `requestSession` resolves and there is a session, and an `end` event
/// says there no longer is; everything else about whether the session is *showing* is `visibilityState`, which
/// is a separate axis and is [`Visibility`] here. So this is three facts: a session is being asked for, there
/// is one, or there was one and it is over.
///
/// What this replaced was OpenXR's ladder, with WebXR mapped down onto it - which is backwards, and for the
/// reason this whole workspace is shaped the way it is. A core that takes the *biggest* platform's vocabulary
/// as its own is a core the other two have to be bent into, and the bend is where the meaning goes missing: a
/// `visible-blurred` session is not a `Synchronized` one, and OpenXR's `VISIBLE` (not focused) is not a
/// distinct rung of a lifecycle at all. OpenXR's own states are its backend's business and stay there, where a
/// program that needs them can ask for them.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum State {
    /// A session has been asked for and is not ready.
    ///
    /// WebXR is why this exists: its `requestSession` is a promise, and a runtime whose session arrives
    /// asynchronously cannot be connected to in one call. A backend that has to wait says so and is polled,
    /// which is the same ladder a session is already climbed by rather than a second mechanism.
    #[default]
    Connecting,
    /// There is a session.
    Ready,
    /// There was a session and it is over.
    Ended,
}

impl State {
    /// Whether the session still exists, in either of the two ways it can.
    pub fn is_alive(self) -> bool {
        !matches!(self, Self::Ended)
    }

    /// Whether the session is still being asked for.
    pub fn is_connecting(self) -> bool {
        matches!(self, Self::Connecting)
    }
}

/// How a session reads its input, which is WebXR's `interactionMode`.
///
/// `WorldSpace` is a session whose input has a place in the scene - a controller, a hand, a gaze ray - and it
/// is what every session drawn into a display is. `ScreenSpace` is a session whose input is a finger or a
/// mouse on a flat surface, which is what an inline session on a phone is. Which one it is belongs to the
/// runtime: an app does not choose it, and the same session calling `inputs` means different things depending
/// on the answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum InteractionMode {
    #[default]
    WorldSpace,
    ScreenSpace,
}

/// Whether the session is being shown, which is WebXR's `visibilityState` and nothing more.
///
/// A separate axis from [`State`] because it is one in the specification: a session exists and is either on a
/// display or not. A core that folded the two together could not say "the session is running and the system
/// menu is over it", which is a thing that happens every time somebody opens one.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Visibility {
    /// There is a session and it is not on a display.
    #[default]
    Hidden,
    /// It is on a display and somebody is looking at it. The state a frame loop draws in.
    Visible,
    /// It is on a display and something else has the person's attention - a system menu, a notification, a
    /// second app in a shared space. A session that keeps drawing and may be drawn less often.
    ///
    /// This is the rung that was being lost: OpenXR's `VISIBLE` without `FOCUSED` is the same fact, and a core
    /// with one ladder had nowhere to put either of them.
    VisibleBlurred,
}

impl Visibility {
    /// Whether the app should draw, which is the question a frame loop is really asking.
    ///
    /// Both of the showing states are a yes: a blurred session is still on a display, and a frame that is not
    /// drawn is a hole in it.
    pub fn can_render(self) -> bool {
        matches!(self, Self::Visible | Self::VisibleBlurred)
    }
}

/// Something that happened to the session.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    StateChanged(State),
    /// The session started or stopped being shown. WebXR's `visibilitychange`, and the other axis: a session
    /// that is shown is not a session that exists.
    VisibilityChanged(Visibility),
    /// The runtime wants to stop - the user took the headset off, or the page lost the session. Not an
    /// error, and not a state: the app is expected to shut its session down cleanly.
    ExitRequested,
    /// The set of inputs changed. What the inputs then *are* is a frame's business.
    InputsChanged,
    /// A reference space's origin was recentered, which is WebXR's `reset`. Every pose measured in it is stale
    /// afterwards - a scene that was placed in the room before the recenter is somewhere else after it.
    Reset(ReferenceSpace),
    /// The primary action started on a source. WebXR's `selectstart`.
    SelectStart(InputId),
    /// It finished. WebXR's `selectend`.
    SelectEnd(InputId),
    /// A selection completed: the press and the release both happened. WebXR's `select`, and it follows the
    /// `SelectEnd` of the same source.
    Select(InputId),
    /// The grip closed. WebXR's `squeezestart`.
    SqueezeStart(InputId),
    /// It opened. WebXR's `squeezeend`.
    SqueezeEnd(InputId),
    /// A squeeze completed. WebXR's `squeeze`.
    Squeeze(InputId),
    /// The session is gone and cannot be resumed.
    Lost,
}

/// What kind of session to ask for, which is WebXR's `XRSessionMode`.
///
/// The three are not three amounts of one thing. `Inline` is a session in the page, with no display of its own
/// and no headset at all; the two immersive ones are a headset, and they differ in what the picture *is* -
/// `ImmersiveVr` where it is the world, `ImmersiveAr` where it is drawn over one. That difference is what a
/// runtime reads to decide whether to offer surfaces and raycasts and the camera, so it has to be asked for
/// before there is a session to ask.
///
/// A platform that decides this itself - OpenXR by the system's configuration, a compositor platform by the
/// immersive space the app made in its own language - takes the mode and answers as though it had been asked
/// in its own terms. It is a request, not a switch.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SessionMode {
    /// A session in the page, with no display of its own. No headset.
    Inline,
    /// A headset, and the picture is the world. The default, because it is what a session is for.
    #[default]
    ImmersiveVr,
    /// A headset, and the picture is drawn over the world.
    ImmersiveAr,
}

/// How the picture reaches the display, which is the one thing the platforms do not agree about.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Presentation {
    /// The app draws into the images the compositor gives it. OpenXR, WebXR.
    Composited,
    /// The platform draws the scene the app describes, and there is no image to draw into. RealityKit, and
    /// anything else that owns its renderer.
    Scene,
}

/// What the display shows behind the picture.
///
/// WebXR's `environmentBlendMode`, and it is in the core for the reason the core is WebXR's vocabulary at all:
/// it changes how a scene has to be drawn, not how a platform is. An opaque display is a screen; an additive
/// one adds the picture to what is already there, so black is see-through and the background is not the app's
/// to fill; an alpha-blending one mixes the two, which is what makes the alpha channel a decision. A renderer
/// that does not know which it is drawing into guesses about the background.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Blend {
    /// The picture replaces what is behind it. Every fully immersive headset.
    #[default]
    Opaque,
    /// The picture is added to the world: black is nothing.
    Additive,
    /// The picture is mixed with the world, so the alpha channel means something.
    AlphaBlend,
}

/// What went wrong.
///
/// Strings rather than platform error types, because the core would otherwise have to depend on every
/// runtime's error enum to say what happened. A backend keeps its own error and hands the sentence on.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no runtime is available: {0}")]
    Unavailable(String),
    #[error("the runtime refused the session: {0}")]
    Rejected(String),
    #[error("the runtime offers no {0:?} space")]
    NoSpace(SpaceKind),
    #[error("this runtime does not do {0}")]
    Unsupported(String),
    #[error("the frame could not be presented: {0}")]
    Present(String),
}
