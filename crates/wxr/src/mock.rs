//! A runtime that is not there.
//!
//! A headset is not needed to test what a session *is*: the frame loop, the state ladder, the eye poses, and
//! the arithmetic that turns a hand in a shoulder into a hand in a room. The mock fabricates all of it, so
//! the core's tests run on a machine with no XR runtime at all - and so a renderer can be developed against
//! a session before any backend exists.

use std::collections::VecDeque;
use std::time::Duration;

use glam::{Quat, Vec3};

use crate::frame::{Eye, FieldOfView, Frame, FrameState, View, Viewport};
use crate::session::{Backend, Error, Event, Presentation, Session, State};
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
    /// The states to hand out, in order, before it settles. Makes the ladder something to test rather than
    /// something to assume.
    pub states: Vec<State>,
}

impl Default for MockBackend {
    fn default() -> Self {
        Self {
            extent: Extent2d::new(2048, 2048),
            stereo: true,
            states: vec![State::Synchronized, State::Visible, State::Focused],
        }
    }
}

impl Backend for MockBackend {
    type Session = MockSession;

    fn connect(&self) -> Result<Self::Session, Error> {
        Ok(MockSession {
            state: State::Idle,
            pending: self
                .states
                .iter()
                .copied()
                .map(Event::StateChanged)
                .collect(),
            extent: self.extent,
            stereo: self.stereo,
            spaces: 0,
            frames: 0,
        })
    }
}

/// A session with no runtime behind it.
///
/// Its `Image` is a number, because there is nothing to name: the point of the mock is that the core never
/// asks what an image *is*.
pub struct MockSession {
    state: State,
    pending: VecDeque<Event>,
    extent: Extent2d,
    stereo: bool,
    spaces: u32,
    frames: u64,
}

impl Session for MockSession {
    type Image = u32;

    fn presentation(&self) -> Presentation {
        Presentation::Composited
    }

    fn state(&self) -> State {
        self.state
    }

    fn poll(&mut self) -> Option<Event> {
        let event = self.pending.pop_front()?;
        if let Event::StateChanged(state) = event {
            self.state = state;
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
        const ONLY: u32 = 0;
        (index == 0).then_some(&ONLY)
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

    fn begin(&mut self, now: Duration, out: &mut Frame) -> Result<(), Error> {
        self.frames += 1;
        out.predicted_display_time = now + PREDICTION;
        out.state = match self.state {
            State::Visible | State::Focused => FrameState::Render,
            State::Stopping | State::Ended => FrameState::Exit,
            _ => FrameState::Wait,
        };
        out.views_mut().clear();
        Ok(())
    }

    fn views(&mut self, space: ReferenceSpace, out: &mut Frame) -> Result<(), Error> {
        if !matches!(space.kind, SpaceKind::LocalFloor | SpaceKind::BoundedFloor) {
            return Err(Error::NoSpace(space.kind));
        }
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
                layer: 0,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive the mock the way an app would: poll to `Visible`, then run frames.
    fn running() -> MockSession {
        let mut session = MockBackend::default().connect().expect("the mock connects");
        while let Some(event) = session.poll() {
            if let Event::StateChanged(State::Focused) = event {
                break;
            }
        }
        session
    }

    #[test]
    fn the_state_ladder_is_climbed_by_polling() {
        let mut session = MockBackend::default().connect().unwrap();
        assert_eq!(session.state(), State::Idle);
        session.poll();
        assert_eq!(session.state(), State::Synchronized);
        session.poll();
        assert_eq!(session.state(), State::Visible);
        session.poll();
        assert_eq!(session.state(), State::Focused);
        assert!(session.state().can_render());
        assert!(session.poll().is_none());
    }

    #[test]
    fn a_frame_has_an_eye_a_side_an_interpupillary_distance_apart() {
        let mut session = running();
        let space = session.space(SpaceKind::LocalFloor).expect("a floor");
        let mut frame = Frame::default();
        session.begin(Duration::ZERO, &mut frame).unwrap();
        session.views(space, &mut frame).unwrap();

        assert!(frame.is_render());
        assert_eq!(frame.views().len(), 2);
        let (left, right) = (&frame.views()[0], &frame.views()[1]);
        assert_eq!(left.eye, Eye::Left);
        assert_eq!(right.eye, Eye::Right);
        let apart = (right.pose.position - left.pose.position).length();
        assert!((apart - IPD).abs() < 1e-6, "{apart}");
        // The eyes are layers of one image, not two images.
        assert_ne!(left.layer, right.layer);
    }

    #[test]
    fn a_space_without_a_floor_is_refused_rather_than_downgraded() {
        let mut session = running();
        assert!(matches!(
            session.space(SpaceKind::Viewer),
            Err(Error::NoSpace(SpaceKind::Viewer))
        ));
    }

    #[test]
    fn the_prediction_is_ahead_of_now_not_behind_it() {
        let mut session = running();
        let mut frame = Frame::default();
        let now = Duration::from_secs(3);
        session.begin(now, &mut frame).unwrap();
        assert!(frame.predicted_display_time > now);
    }

    #[test]
    fn a_stopped_session_asks_to_exit() {
        let mut session = MockBackend {
            states: vec![State::Visible, State::Stopping],
            ..Default::default()
        }
        .connect()
        .unwrap();
        session.poll();
        session.poll();
        let mut frame = Frame::default();
        session.begin(Duration::ZERO, &mut frame).unwrap();
        assert_eq!(frame.state, FrameState::Exit);
    }
}
