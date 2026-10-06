//! A runtime, connected and then running.

use std::time::Duration;

use crate::frame::Frame;
use crate::space::{ReferenceSpace, SpaceKind};
use crate::target::ImageMeta;

/// What the runtime is doing.
///
/// WebXR's states, and they are a chain rather than a flag: a session that is not yet `Visible` has nothing
/// to draw into, and a session past `Visible` has stopped drawing. Naming them the same way the spec does
/// means a backend can be read against its own documentation.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum State {
    /// Connected, with no session yet.
    #[default]
    Idle,
    /// A session exists and has not been asked to run.
    Ready,
    /// Running, and not yet showing anything.
    Synchronized,
    /// Showing, and the app may draw.
    Visible,
    /// Showing and receiving input: the user is in it.
    Focused,
    /// Being torn down.
    Stopping,
    /// Gone.
    Ended,
}

impl State {
    /// Whether the app may draw into this session's images.
    pub fn can_render(self) -> bool {
        matches!(self, Self::Visible | Self::Focused)
    }

    /// Whether the session is still worth polling.
    pub fn is_alive(self) -> bool {
        !matches!(self, Self::Ended)
    }
}

/// Something that happened to the session.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    StateChanged(State),
    /// The runtime wants to stop - the user took the headset off, or the page lost the session. Not an
    /// error, and not a state: the app is expected to shut its session down cleanly.
    ExitRequested,
    /// The set of inputs changed. What the inputs then *are* is a frame's business.
    InputsChanged,
    /// The session is gone and cannot be resumed.
    Lost,
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
    #[error("the frame could not be presented: {0}")]
    Present(String),
}

/// An XR runtime that can be connected to.
pub trait Backend {
    type Session: Session;

    /// Connect, which is not the same as running: the session that comes back is `Idle`.
    fn connect(&self) -> Result<Self::Session, Error>;
}

/// A live session.
///
/// The images are an associated type and not a handle, because the three platforms do not agree on what
/// one is: a `VkImage` on one, an object that only exists once it is bound to a device on another, and
/// nothing at all on the third. The renderer's importer is the code that knows which, and it is written per
/// backend - which is what keeps this trait free of any graphics API's name.
pub trait Session {
    /// The compositor's own name for an image. Opaque here, on purpose.
    type Image;

    fn presentation(&self) -> Presentation;
    fn state(&self) -> State;

    /// The next thing that happened, or `None` if nothing has. Drained one at a time, because two events in
    /// a row can matter in the order they happened - `Visible` then `ExitRequested` is a session that ran
    /// and stopped.
    fn poll(&mut self) -> Option<Event>;

    /// The shape of every image this session presents.
    fn images(&self) -> ImageMeta;

    /// How many images there are: one per eye for a compositor that does not layer them, one when it does.
    fn image_count(&self) -> usize;

    /// The image at `index`, as this platform names it.
    fn image(&self, index: usize) -> Option<&Self::Image>;

    /// Ask for a space to measure poses in. A runtime that does not have the one asked for says so, rather
    /// than quietly handing back a worse one - a scene that asked to stand on the floor and got a head
    /// origin is a scene floating at eye height.
    fn space(&mut self, kind: SpaceKind) -> Result<ReferenceSpace, Error>;

    /// Begin a frame. Fills in the predicted display time and what the caller should do with it.
    fn begin(&mut self, now: Duration, out: &mut Frame) -> Result<(), Error>;

    /// Fill in the views for the frame just begun, in the space given.
    fn views(&mut self, space: ReferenceSpace, out: &mut Frame) -> Result<(), Error>;

    /// Hand the frame back for the compositor to present.
    fn end(&mut self, frame: &mut Frame) -> Result<(), Error>;
}

/// Whether a session is both able and expected to draw, which is the question a renderer really asks.
pub fn should_render<S: Session + ?Sized>(session: &S) -> bool {
    session.presentation() == Presentation::Composited && session.state().can_render()
}
