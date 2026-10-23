//! What a session is, on a machine that has no runtime to ask.
//!
//! The bodies of `MockSession`'s `Session` implementation, which is the longest single thing the mock does
//! and the part a test reads the least: a test asks what a session *answered*, and the answers a test can
//! check are the ones in `accessors`. This is the shape they answer with.

use super::*;

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
        // The floor is a session and nothing else. Two capabilities are asked for: a quad layer, and hands -
        // both are off by default, which is what makes each a thing to test against rather than an assumption.
        // Haptics and meshes are always there because the mock's controllers have actuators and its room has a
        // table: a caller that branches on a bit has something to branch into on the backend it develops
        // against, which is what testing needs.
        Features::HAPTICS
            .union(Features::MESH)
            .union(if self.quad_layers {
                Features::LAYER_QUAD
            } else {
                Features::NONE
            })
            .union(if self.hands {
                Features::HAND_TRACKING
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
        // The caller's vector is the frame loop's, so it holds the last frame's sources until this says
        // otherwise - and two sources a frame is a list that is mostly the past.
        out.clear();
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
                // A mock's two sources are what the backend was told they are: hands when it was told to have
                // them, and controllers when it was not. A skeleton that is always there would be a mock that
                // lets a game ship code no floor-level session has.
                hand: self.hands,
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

    fn hand(
        &mut self,
        source: InputId,
        _space: ReferenceSpace,
        out: &mut Hand,
    ) -> Result<(), Error> {
        // Asked for whoever said it had one, and recorded either way. A backend that asked a controller would
        // be a backend doing a round trip for an answer it already knew, and this is the call that shows it.
        self.hand_asks.push(source);
        out.clear();
        if !self.hands {
            return Ok(());
        }
        // A wrist and a fingertip, two sides of a line: the smallest skeleton that has a direction, which is
        // what a caller drawing a finger has to be able to see. The left is on the left of the right, so that
        // two sources in one frame are two answers rather than one answer twice.
        let x = if source.get() == 0 { -0.2 } else { 0.2 };
        let joints = out.joints_mut();
        for joint in [HandJoint::Wrist, HandJoint::IndexFingerTip] {
            let ahead = if joint == HandJoint::Wrist {
                0.0
            } else {
                -0.08
            };
            joints[joint.index()] = Some(Joint {
                pose: Pose {
                    position: Vec3::new(x, 1.0, ahead),
                    orientation: Quat::IDENTITY,
                },
                radius: if joint == HandJoint::Wrist {
                    0.05
                } else {
                    0.01
                },
            });
        }
        Ok(())
    }

    fn meshes(&mut self, _space: ReferenceSpace, out: &mut Vec<Mesh>) -> Result<(), Error> {
        // One flat square a metre across, a metre up: a mock's room is the smallest thing that has the shape
        // of one - triangles, a place, and a label - so a caller that reads meshes has something to read on
        // the backend it is developed against.
        out.clear();
        let half = 0.5;
        out.push(Mesh {
            id: 0,
            pose: Pose {
                position: Vec3::new(0.0, 1.0, 0.0),
                orientation: Quat::IDENTITY,
            },
            vertices: vec![
                Vec3::new(-half, 0.0, -half),
                Vec3::new(half, 0.0, -half),
                Vec3::new(half, 0.0, half),
                Vec3::new(-half, 0.0, half),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            kind: MeshKind::Table,
            last_changed: Duration::ZERO,
        });
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
