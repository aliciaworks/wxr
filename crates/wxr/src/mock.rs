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
use crate::layer::{Layer, LayerImage, LayerShape};
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
            spaces: 0,
            frames: 0,
            image: 0,
            layers: Vec::new(),
            pulses: Vec::new(),
            waveforms: Vec::new(),
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
    /// `None` for one that was released, so that a test can tell the difference between a layer that is gone and
    /// one that was never there - which is the difference `release_layer` exists to make.
    layers: Vec<Option<MockLayer>>,
    /// Every vibration asked for, oldest first: what source, how hard, and for how long. A mock records
    /// because a `pulse` that returns `Ok` and does nothing is indistinguishable from one that failed.
    pulses: Vec<(InputId, f32, Duration)>,
    /// Every waveform asked for, as the source, how many samples, and the rate - the samples themselves are
    /// not kept, because what a test asks is whether the call happened and with what.
    waveforms: Vec<(InputId, usize, f32)>,
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
        // The floor is a session and nothing else; a mock told to have a quad layer says so, and that is the
        // only capability it can have - which is what makes it a thing to test against. Haptics is the second,
        // and it is always there because the mock's controllers are made with actuators: a caller that branches
        // on the bit has something to branch into on this backend, which is what testing needs.
        Features::HAPTICS.union(if self.quad_layers {
            Features::LAYER_QUAD
        } else {
            Features::NONE
        })
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

    fn layer(
        &mut self,
        space: ReferenceSpace,
        shape: LayerShape,
        pixels: Extent2d,
    ) -> Result<Layer, Error> {
        // One shape, and the refusal is the interesting half: a mock that quietly made a cylinder would be a
        // mock that let a renderer ship code for a capability no session here has.
        if !self.quad_layers || !matches!(shape, LayerShape::Quad { .. }) {
            return Err(Error::Unsupported(format!("{} layers", shape.name())));
        }
        self.layers.push(Some(MockLayer {
            shape,
            space,
            pose: Pose::IDENTITY,
            pixels,
            image: 0,
        }));
        Ok(Layer::new((self.layers.len() - 1) as u32))
    }

    fn layer_image(&mut self, layer: Layer) -> Option<(&Self::Image, LayerImage)> {
        let frames = self.frames;
        let slot = self.layers.get_mut(layer.id() as usize)?.as_mut()?;
        // Unlike the session's own image, this one carries its name as well: two layers are two pictures, and a
        // renderer handed the same handle for both would draw one of them twice.
        slot.image = (frames + layer.id() as u64) as u32;
        let image = &slot.image;
        let meta = ImageMeta {
            format: ColorFormat::Rgba8Srgb,
            extent: slot.pixels,
            // One: a quad is one picture for both eyes, which is the whole reason to hand it to a compositor.
            layers: 1,
        };
        Some((
            image,
            LayerImage {
                meta,
                viewport: Viewport {
                    width: meta.extent.width,
                    height: meta.extent.height,
                    ..Default::default()
                },
            },
        ))
    }

    fn set_layer_pose(&mut self, layer: Layer, pose: Pose) -> Result<(), Error> {
        let Some(slot) = self
            .layers
            .get_mut(layer.id() as usize)
            .and_then(Option::as_mut)
        else {
            return Err(Error::Unsupported("no such layer".into()));
        };
        slot.pose = pose;
        Ok(())
    }

    fn release_layer(&mut self, layer: Layer) {
        if let Some(slot) = self.layers.get_mut(layer.id() as usize) {
            *slot = None;
        }
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
                // And it can buzz, because a mock with no actuator would be a mock that cannot exercise the
                // two calls that send one - which is the whole reason the bit exists.
                haptics: true,
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

    fn pulse(&mut self, source: InputId, intensity: f32, duration: Duration) -> Result<(), Error> {
        // Clamped, not refused: the core's word is a request, and a mock that rejected an out-of-range
        // intensity would be a mock that behaves unlike every platform.
        self.pulses
            .push((source, intensity.clamp(0.0, 1.0), duration));
        Ok(())
    }

    fn play_pcm(
        &mut self,
        source: InputId,
        samples: &[f32],
        sample_rate: f32,
    ) -> Result<(), Error> {
        self.waveforms.push((source, samples.len(), sample_rate));
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

    /// Every vibration asked for, oldest first, as the source, the intensity and the length.
    pub fn pulses(&self) -> &[(InputId, f32, Duration)] {
        &self.pulses
    }

    /// Every waveform asked for, oldest first, as the source, how many samples, and the rate.
    pub fn waveforms(&self) -> &[(InputId, usize, f32)] {
        &self.waveforms
    }

    /// Pretend a space's origin was recentered, which is the one thing a backend that cannot recenter still
    /// has to be able to pass on.
    pub fn reset(&mut self, space: ReferenceSpace) {
        self.pending.push_back(Event::Reset(space));
    }

    /// How many layers are live, so a test can tell a release from a leak.
    pub fn layer_count(&self) -> usize {
        self.layers.iter().filter(|layer| layer.is_some()).count()
    }

    /// Where a layer was last placed, which is how a test sees that placing it did something.
    pub fn layer_pose(&self, layer: Layer) -> Option<Pose> {
        self.layers
            .get(layer.id() as usize)
            .and_then(Option::as_ref)
            .map(|slot| slot.pose)
    }

    /// The shape and the resolution a layer was made with, which the core deliberately does not hand back: a
    /// caller that asked knows, and a test that asks is checking that it was kept.
    pub fn layer_shape(&self, layer: Layer) -> Option<(LayerShape, Extent2d)> {
        self.layers
            .get(layer.id() as usize)
            .and_then(Option::as_ref)
            .map(|slot| (slot.shape, slot.pixels))
    }

    /// The space a layer was made in.
    pub fn layer_space(&self, layer: Layer) -> Option<ReferenceSpace> {
        self.layers
            .get(layer.id() as usize)
            .and_then(Option::as_ref)
            .map(|slot| slot.space)
    }
}

#[cfg(test)]
mod tests;
