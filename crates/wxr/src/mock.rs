//! A runtime that is not there.
//!
//! A headset is not needed to test what a session *is*: the frame loop, the two ladders a session climbs, the
//! eye poses, and the arithmetic that turns a hand in a shoulder into a hand in a room. The mock fabricates
//! all of it, so the core's tests run on a machine with no XR runtime at all - and so a renderer can be
//! developed against a session before any backend exists.

use std::collections::VecDeque;
use std::time::Duration;

use glam::{Quat, Vec3};

use crate::feature::Features;
use crate::frame::{Eye, FieldOfView, Frame, FrameState, View, Viewport};
use crate::input::{
    Axes, Buttons, Hand, HandJoint, Handedness, InputId, InputSource, Joint, TargetRayMode,
};
use crate::layer::{Layer, LayerImage, LayerShape};
use crate::mesh::{Mesh, MeshKind};
use crate::session::{
    Backend, Error, Event, Presentation, Session, SessionMode, State, Visibility,
};
use crate::space::{Pose, ReferenceSpace, SpaceKind};
use crate::target::{ColorFormat, Extent2d, ImageMeta};

/// How far apart the mock eyes are: a real interpupillary distance, near enough to be worth rendering.
pub const IPD: f32 = 0.063;

/// How far ahead of the frame the mock predicts the display, which is what a real prediction is for.
pub const PREDICTION: Duration = Duration::from_micros(11_000);

/// A backend that invents a session.
#[derive(Clone, Debug)]
pub struct MockBackend {
    pub extent: Extent2d,
    pub stereo: bool,
    /// The lifecycle rungs to hand out, in order. `Ready` by default, and on its own, because that is what
    /// WebXR's promise does: the session arrives and that is the whole of it.
    pub states: Vec<State>,
    /// The visibility rungs to hand out after them, in order. What makes the other ladder something to test
    /// rather than something to assume - and the rung that a core with one ladder had nowhere to put.
    pub visibility: Vec<Visibility>,
    /// Whether this mock has a quad layer to hand out, which it does not by default: a session with no
    /// capabilities is the floor every backend shares, and the one the mock is by default so that a test of
    /// "a session without the feature" has a session without the feature.
    pub quad_layers: bool,
    /// Whether the two sources are hands with skeletons behind them, which they are not by default for the
    /// same reason the layers are not: a session without hand tracking is the floor, and a test that wants
    /// fingers asks for them.
    ///
    /// A hand and a controller cannot both be true of one source, so a mock with hands makes its two sources
    /// hands - which is also what makes two ids worth having, since the joints of the left and the right are
    /// in different places.
    pub hands: bool,
}

impl Default for MockBackend {
    fn default() -> Self {
        Self {
            extent: Extent2d::new(2048, 2048),
            stereo: true,
            states: vec![State::Ready],
            visibility: vec![
                Visibility::Hidden,
                Visibility::Visible,
                Visibility::VisibleBlurred,
            ],
            quad_layers: false,
            hands: false,
        }
    }
}

impl Backend for MockBackend {
    /// Nothing to hand over: there is no device, because there is no renderer.
    type Device = ();
    type Session = MockSession;

    fn connect(&self, _device: (), mode: SessionMode) -> Result<Self::Session, Error> {
        // Two ladders, in the order they happen: the session arrives, and then it is shown.
        let pending: VecDeque<Event> = self
            .states
            .iter()
            .copied()
            .map(Event::StateChanged)
            .chain(
                self.visibility
                    .iter()
                    .copied()
                    .map(Event::VisibilityChanged),
            )
            .collect();
        Ok(MockSession {
            mode,
            state: State::Connecting,
            visibility: Visibility::Hidden,
            pending,
            extent: self.extent,
            stereo: self.stereo,
            quad_layers: self.quad_layers,
            hands: self.hands,
            spaces: 0,
            frames: 0,
            image: 0,
            layers: Vec::new(),
            pulses: Vec::new(),
            waveforms: Vec::new(),
            hand_asks: Vec::new(),
        })
    }
}

/// A layer the mock is holding, so that placing one and drawing into it are things a test can see.
struct MockLayer {
    shape: LayerShape,
    space: ReferenceSpace,
    pose: Pose,
    /// What the app asked to draw it at, which is what comes back as the image's extent.
    pixels: Extent2d,
    /// This frame's image, which changes the way the session's own does and for the same reason.
    image: u32,
}

/// A session with no runtime behind it.
///
/// Its `Image` is a number, because there is nothing to name: the point of the mock is that the core never
/// asks what an image *is*.
pub struct MockSession {
    mode: SessionMode,
    state: State,
    visibility: Visibility,
    pending: VecDeque<Event>,
    extent: Extent2d,
    stereo: bool,
    spaces: u32,
    frames: u64,
    /// The image this frame hands out. It changes every frame because a compositor's handles do - a drawable's
    /// textures, a swapchain's acquired image - and a mock that handed out one handle forever would let a
    /// renderer cache it and still look right.
    image: u32,
    quad_layers: bool,
    /// Whether the sources are hands, which is what makes `hand` answer with something rather than with an
    /// empty skeleton. A session without it is the floor, and the mock's default.
    hands: bool,
    /// `None` for one that was released, so that a test can tell the difference between a layer that is gone and
    /// one that was never there - which is the difference `release_layer` exists to make.
    layers: Vec<Option<MockLayer>>,
    /// Every vibration asked for, oldest first: what source, how hard, and for how long. A mock records
    /// because a `pulse` that returns `Ok` and does nothing is indistinguishable from one that failed.
    pulses: Vec<(InputId, f32, Duration)>,
    /// Every waveform asked for, as the source, how many samples, and the rate - the samples themselves are
    /// not kept, because what a test asks is whether the call happened and with what.
    waveforms: Vec<(InputId, usize, f32)>,
    /// Every skeleton asked for, oldest first. Recorded for the same reason the vibrations are - but the other
    /// way round: asking a session for a hand that says it has none is a call that should not have been made,
    /// and the only way to see it is to count who was asked.
    hand_asks: Vec<InputId>,
}

mod session;

#[cfg(test)]
mod tests;

mod accessors;
