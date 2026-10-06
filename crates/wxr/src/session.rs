//! A runtime, connected and then running.

use std::any::Any;
use std::time::Duration;

use crate::frame::Frame;
use crate::input::{InputId, InputSource};
use crate::space::{Pose, ReferenceSpace, SpaceKind};
use crate::target::ImageMeta;

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
    #[error("the frame could not be presented: {0}")]
    Present(String),
}

/// An XR runtime that can be connected to.
pub trait Backend {
    /// What the renderer has to hand over.
    ///
    /// An associated type for the same reason [`Session::Image`] is one: a Vulkan device, a D3D12 device
    /// and nothing at all are not one thing. What it says is *who makes it* - the renderer does, and the
    /// backend is told afterwards. The other order, where the runtime makes a device for the renderer to
    /// adopt, is how an XR layer ends up owning every graphics decision above it.
    type Device;
    type Session: Session;

    /// Connect, which is not the same as running: the session that comes back is `Idle`.
    fn connect(&self, device: Self::Device) -> Result<Self::Session, Error>;
}

/// A live session.
///
/// The images are an associated type and not a handle, because the three platforms do not agree on what
/// one is: a `VkImage` on one, an object that only exists once it is bound to a device on another, and
/// nothing at all on the third. The renderer's importer is the code that knows which, and it is written per
/// backend - which is what keeps this trait free of any graphics API's name.
///
/// It is [`Any`] for one reason, and it is the same reason [`Session::Image`] is an associated type: the core
/// is a subset, and a subset has to be able to hand back what it left out. [`Session::as_backend`] is that
/// seam.
pub trait Session: Any {
    /// The compositor's own name for an image. Opaque here, on purpose.
    type Image;

    /// The backend's own session, when a program needs the platform the core deliberately does not speak.
    ///
    /// This is the core's answer to `wgpu`'s `as_hal` - the same escape hatch, for the same reason. The core
    /// says exactly what WebXR says and nothing else, which is what lets three unrelated platforms share it;
    /// the price is that anything only one of them has is not here, so a program that needs OpenXR's own eight
    /// session states, or WebXR's `XRGPUBinding`, or a `cp_drawable`, needs a way back to the type that does.
    ///
    /// What comes back is *not* this API. `None` means "a different backend", and a backend's own types are
    /// free to change in a way the core's vocabulary is not. Reach for [`Session::visibility`] first, and
    /// through here only for what the core has decided it will not say.
    ///
    /// ```
    /// use wxr::{Backend as _, Session as _};
    ///
    /// let session = wxr::mock::MockBackend::default().connect(()).unwrap();
    /// // The core's vocabulary...
    /// assert_eq!(session.state(), wxr::State::Connecting);
    /// // ...and the backend's own type, when the platform is what matters.
    /// let mock = session.as_backend::<wxr::mock::MockSession>().unwrap();
    /// assert_eq!(mock.state(), wxr::State::Connecting);
    /// ```
    fn as_backend<T: 'static>(&self) -> Option<&T>
    where
        Self: Sized,
    {
        (self as &dyn Any).downcast_ref()
    }

    /// The same, mutably, for the part of a platform a program sets rather than reads.
    fn as_backend_mut<T: 'static>(&mut self) -> Option<&mut T>
    where
        Self: Sized,
    {
        (self as &mut dyn Any).downcast_mut()
    }

    fn presentation(&self) -> Presentation;

    /// The session's lifecycle: is it being asked for, is it here, is it over.
    fn state(&self) -> State;

    /// Whether the session is being shown.
    ///
    /// Hidden by default, because a backend that does not say is one whose display nobody has looked at yet -
    /// and because drawing into a session that is not shown is drawing into nothing.
    fn visibility(&self) -> Visibility {
        Visibility::Hidden
    }

    /// The next thing that happened, or `None` if nothing has. Drained one at a time, because two events in
    /// a row can matter in the order they happened - `Visible` then `ExitRequested` is a session that ran
    /// and stopped.
    fn poll(&mut self) -> Option<Event>;

    /// What the display shows behind the picture.
    ///
    /// `Opaque` by default, because a session that does not say is a session that fills the display - which is
    /// what both ways of presenting into one do.
    fn blend(&self) -> Blend {
        Blend::Opaque
    }

    /// The near and far planes the scene draws with, in metres.
    ///
    /// This is WebXR's `XRRenderState.depthNear` and `depthFar`, and it is in the core for the reason the core
    /// is WebXR's vocabulary: a compositor that reprojects a frame with the depth buffer cannot read the planes
    /// off the picture, so the app has to say what the values in it mean. `wgpu`'s depth is centred on the
    /// same range on all three platforms, so this is the one number a renderer and a compositor have to agree
    /// on.
    ///
    /// Nothing by default, and that is not a gap: a backend with no depth to submit has no planes to set. A
    /// backend that does submit depth overrides this.
    fn set_depth_range(&mut self, _near: f32, _far: f32) {}

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

    /// A space at `offset` inside `space`, for content that belongs to a place rather than to a room.
    ///
    /// This is WebXR's `getOffsetReferenceSpace`, and it is how a scene anchors something: a panel on a wall, a
    /// model on a table, a point the app decided is "here". The offset is expressed in `space`, so what comes
    /// back moves with it - a space off a controller follows the controller, and one off the floor stays put.
    ///
    /// A backend makes this out of what it already has: OpenXR a reference space of the same type with a pose
    /// inside it, WebXR the browser's own offset space, and a compositor platform a transform it applies where
    /// it would have applied the one it had.
    fn offset_space(
        &mut self,
        space: ReferenceSpace,
        offset: Pose,
    ) -> Result<ReferenceSpace, Error>;

    /// Begin a frame. Fills in the predicted display time and what the caller should do with it.
    fn begin(&mut self, now: Duration, out: &mut Frame) -> Result<(), Error>;

    /// Fill in the views for the frame just begun, in the space given.
    fn views(&mut self, space: ReferenceSpace, out: &mut Frame) -> Result<(), Error>;

    /// Fill in what the user is holding, in the space given.
    ///
    /// Empty by default, because a backend that has no inputs yet - a runtime with no controllers bound, a
    /// scene being watched rather than played - is a backend with nothing to report rather than one that
    /// failed.
    fn inputs(&mut self, _space: ReferenceSpace, _out: &mut Vec<InputSource>) -> Result<(), Error> {
        Ok(())
    }

    /// Hand the frame back for the compositor to present.
    fn end(&mut self, frame: &mut Frame) -> Result<(), Error>;
}

/// Whether a session is both able and expected to draw, which is the question a renderer really asks.
pub fn should_render<S: Session + ?Sized>(session: &S) -> bool {
    session.presentation() == Presentation::Composited && session.visibility().can_render()
}
