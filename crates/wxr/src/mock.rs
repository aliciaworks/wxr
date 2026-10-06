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
use crate::input::{Axes, Buttons, Handedness, InputId, InputSource, TargetRayMode};
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
            spaces: 0,
            frames: 0,
            image: 0,
        })
    }
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
}

impl Session for MockSession {
    type Image = u32;
    type Depth = ();

    fn presentation(&self) -> Presentation {
        Presentation::Composited
    }

    fn state(&self) -> State {
        self.state
    }

    fn visibility(&self) -> Visibility {
        self.visibility
    }

    fn features(&self) -> Features {
        // A mock has a session and nothing else, which is every backend's floor.
        Features::NONE
    }

    fn poll(&mut self) -> Option<Event> {
        let event = self.pending.pop_front()?;
        match event {
            Event::StateChanged(state) => self.state = state,
            Event::VisibilityChanged(visibility) => self.visibility = visibility,
            _ => {}
        }
        Some(event)
    }

    fn images(&self) -> ImageMeta {
        ImageMeta {
            format: ColorFormat::Rgba8Srgb,
            extent: self.extent,
            // A layered target puts an eye in a layer; otherwise each eye is an image of its own.
            layers: if self.stereo { 2 } else { 1 },
        }
    }

    fn image_count(&self) -> usize {
        // One image either way: stereo is two layers of it, which is what makes `layers` worth reporting.
        1
    }

    fn image(&self, index: usize) -> Option<&Self::Image> {
        (index == 0).then_some(&self.image)
    }

    fn space(&mut self, kind: SpaceKind) -> Result<ReferenceSpace, Error> {
        if !kind.has_floor() {
            // The mock has no floor, and saying so is the behaviour worth testing: a core that silently got
            // a head origin where it asked for a floor would put the scene at eye height.
            return Err(Error::NoSpace(kind));
        }
        self.spaces += 1;
        Ok(ReferenceSpace::new(kind, self.spaces))
    }

    fn offset_space(
        &mut self,
        space: ReferenceSpace,
        _offset: Pose,
    ) -> Result<ReferenceSpace, Error> {
        // Every mock space is the same place, so an offset one is a second name for it - which is enough to
        // hold one, and holding one is the thing a caller is testing.
        self.spaces += 1;
        Ok(ReferenceSpace::new(space.kind, self.spaces))
    }

    fn begin(&mut self, now: Duration, out: &mut Frame) -> Result<(), Error> {
        self.frames += 1;
        // Three, like a swapchain, so that an image is reused before long and a cached one would be visibly
        // the wrong one.
        self.image = (self.frames % 3) as u32;
        out.predicted_display_time = now + PREDICTION;
        out.state = match self.state {
            // A session that is over is a loop to stop; one that is being asked for is a frame to wait for; and
            // one that is here draws exactly when it is being shown.
            State::Ended => FrameState::Exit,
            State::Connecting => FrameState::Wait,
            State::Ready => match self.visibility {
                Visibility::Hidden => FrameState::Wait,
                Visibility::Visible | Visibility::VisibleBlurred => FrameState::Render,
            },
        };
        out.views_mut().clear();
        Ok(())
    }

    fn views(&mut self, space: ReferenceSpace, out: &mut Frame) -> Result<(), Error> {
        if !matches!(space.kind, SpaceKind::LocalFloor | SpaceKind::BoundedFloor) {
            return Err(Error::NoSpace(space.kind));
        }
        // The head is between the eyes and the eyes are level with it, which is the one place a mock can be
        // exactly like a real runtime.
        out.viewer = Pose::IDENTITY;
        let fov = FieldOfView::symmetric(std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
        let views = out.views_mut();
        if self.stereo {
            views.push(View {
                eye: Eye::Left,
                pose: Pose {
                    position: Vec3::new(-IPD * 0.5, 0.0, 0.0),
                    orientation: Quat::IDENTITY,
                },
                fov,
                viewport: Viewport {
                    width: self.extent.width,
                    height: self.extent.height,
                    ..Default::default()
                },
                image: 0,
                recommended_viewport_scale: None,
                layer: 0,
            });
            views.push(View {
                eye: Eye::Right,
                pose: Pose {
                    position: Vec3::new(IPD * 0.5, 0.0, 0.0),
                    orientation: Quat::IDENTITY,
                },
                fov,
                viewport: Viewport {
                    width: self.extent.width,
                    height: self.extent.height,
                    ..Default::default()
                },
                image: 0,
                recommended_viewport_scale: None,
                layer: 1,
            });
        } else {
            views.push(View {
                eye: Eye::Mono,
                pose: Pose::IDENTITY,
                fov,
                viewport: Viewport {
                    width: self.extent.width,
                    height: self.extent.height,
                    ..Default::default()
                },
                image: 0,
                recommended_viewport_scale: None,
                layer: 0,
            });
        }
        Ok(())
    }

    fn inputs(&mut self, _space: ReferenceSpace, out: &mut Vec<InputSource>) -> Result<(), Error> {
        // Two controllers, because a mock's job is to be *a* session rather than a plausible one - and because
        // two is what makes an id worth having: an event has to be able to say which.
        for (id, handedness, x) in [
            (0u32, Handedness::Left, -0.2f32),
            (1u32, Handedness::Right, 0.2f32),
        ] {
            out.push(InputSource {
                id: InputId::new(id),
                handedness,
                target_ray_mode: TargetRayMode::TrackedPointer,
                // A mock controller, which is what a mock can be without inventing a skeleton: the joints are
                // what a *hand* has, and the mock is the thing a game is developed against before either exists.
                hand: false,
                grip: Pose {
                    position: Vec3::new(x, 1.0, 0.0),
                    orientation: Quat::IDENTITY,
                },
                aim: Pose {
                    position: Vec3::new(x, 1.0, -0.5),
                    orientation: Quat::IDENTITY,
                },
                tracked: true,
                buttons: Buttons::default(),
                axes: Axes::default(),
            });
        }
        Ok(())
    }

    fn end(&mut self, _frame: &mut Frame) -> Result<(), Error> {
        Ok(())
    }
}

impl MockSession {
    /// How many frames have been begun, so a test can tell the loop ran.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// The mode the session was asked for, which a mock can hold even though nothing acts on it.
    pub fn mode(&self) -> SessionMode {
        self.mode
    }

    /// Pretend the primary action was pressed and released on `id`.
    ///
    /// A real runtime makes these events out of a controller; a test needs them without one, and they are the
    /// whole shape a game reacts to - a start, an end, and the selection that completed.
    pub fn press(&mut self, id: InputId) {
        self.pending.push_back(Event::SelectStart(id));
        self.pending.push_back(Event::SelectEnd(id));
        self.pending.push_back(Event::Select(id));
    }

    /// Pretend something was picked up or put down, which is the set of inputs changing.
    pub fn inputs_changed(&mut self) {
        self.pending.push_back(Event::InputsChanged);
    }

    /// Pretend a space's origin was recentered, which is the one thing a backend that cannot recenter still
    /// has to be able to pass on.
    pub fn reset(&mut self, space: ReferenceSpace) {
        self.pending.push_back(Event::Reset(space));
    }
}

#[cfg(test)]
mod tests;
