//! What this session does when the core asks it something: the `wxr::Session` implementation.
//!
//! It is one `impl` and therefore one file - Rust allows a type one implementation of a trait - and it is
//! separated from the type it implements because the two are read for different reasons: the type and its
//! own methods are what this backend *is*, and this is every answer it gives.

use super::*;

impl wxr::Session for AppleSession {
    type Image = FrameImage;
    /// No depth to hand over: a `CompositorServices` drawable has a depth buffer, but it is the one being
    /// *drawn into* rather than a measurement of the room.
    type Depth = ();

    fn presentation(&self) -> wxr::Presentation {
        wxr::Presentation::Composited
    }

    fn state(&self) -> wxr::State {
        self.layer_state().0
    }

    fn visibility(&self) -> wxr::Visibility {
        self.layer_state().1
    }

    fn features(&self) -> wxr::Features {
        // ARKit's world tracking is a reference space rather than a capability, and its hands are a palm with
        // no skeleton. Surfaces it does have - the plane detection provider - and that is the one bit here.
        let mut features = wxr::Features::NONE;
        if self.arkit.as_ref().is_some_and(ArKit::has_planes) {
            features = features.union(wxr::Features::PLANES);
        }
        if self.arkit.as_ref().is_some_and(ArKit::has_anchors) {
            features = features.union(wxr::Features::ANCHORS);
        }
        features
    }

    fn poll(&mut self) -> Option<wxr::Event> {
        let (state, visibility) = self.layer_state();
        if state != self.reported.0 {
            self.reported.0 = state;
            return Some(wxr::Event::StateChanged(state));
        }
        if visibility != self.reported.1 {
            self.reported.1 = visibility;
            return Some(wxr::Event::VisibilityChanged(visibility));
        }
        if self.inputs_changed {
            self.inputs_changed = false;
            return Some(wxr::Event::InputsChanged);
        }
        None
    }

    fn blend(&self) -> wxr::Blend {
        self.blend
    }

    /// The near and far planes the scene draws with.
    ///
    /// The depth buffer this compositor hands over is only useful to it if it knows what the values in it mean,
    /// and it cannot read that off the picture - so the app says, in metres. The compositor uses it to
    /// reproject the frame: when the prediction was off it moves the pixels to where the head turned out to
    /// be, and depth is what tells it how far each of them is. This is the platform the core's depth range was
    /// written for, and the one where it is already put to work.
    fn set_depth_range(&mut self, near: f32, far: f32) {
        self.depth_range = Some((near, far));
    }

    fn images(&self) -> wxr::ImageMeta {
        // The texture is the truth about itself: the layer's configuration is what the app *asked* for, and
        // the compositor is free to hand back something else - so the format, the size and the layer count are
        // read off the thing that will actually be drawn into, when there is one.
        //
        // Before a frame there is only the configuration, and answering with its format rather than with
        // `Unknown` is what lets a caller build its renderer up front. The extent is a frame's to know and is
        // left at zero until then; nothing draws into it before a drawable says how big it is.
        let Some(image) = self.textures.first() else {
            return wxr::ImageMeta {
                format: self.configured,
                ..Default::default()
            };
        };
        let texture = &image.color;
        let (format, width, height, layers) = (
            texture.pixelFormat(),
            texture.width() as u32,
            texture.height() as u32,
            texture.arrayLength() as u32,
        );
        wxr::ImageMeta {
            format: color_format(format),
            extent: wxr::Extent2d::new(width, height),
            layers,
        }
    }

    fn image_count(&self) -> usize {
        // A compositor may dedicate a texture to each view or layer the views in one; the drawable's
        // texture count is the number either way, and a view's texture map is what says which is which.
        self.textures.len()
    }

    fn image(&self, index: usize) -> Option<&Self::Image> {
        self.textures.get(index)
    }

    fn space(&mut self, kind: wxr::SpaceKind) -> Result<wxr::ReferenceSpace, wxr::Error> {
        // Two spaces this platform has: the device's own, where each eye is relative to the wearer, and -
        // when ARKit came up - the origin it tracks, which is fixed where the session began and is exactly
        // `Local`. A floor is neither: ARKit's origin is not a plane, so a scene that asks to stand on one
        // is told no rather than put at eye height.
        match (kind, &self.arkit) {
            (wxr::SpaceKind::Viewer, _) => {}
            (wxr::SpaceKind::Local, Some(arkit)) if arkit.is_world_tracked() => {}
            _ => return Err(wxr::Error::NoSpace(kind)),
        }
        self.spaces.push(wxr::Pose::IDENTITY);
        Ok(wxr::ReferenceSpace::new(
            kind,
            (self.spaces.len() - 1) as u32,
        ))
    }

    fn offset_space(
        &mut self,
        base: wxr::ReferenceSpace,
        offset: wxr::Pose,
    ) -> Result<wxr::ReferenceSpace, wxr::Error> {
        let Some(inside) = self.spaces.get(base.id() as usize).copied() else {
            return Err(wxr::Error::NoSpace(base.kind));
        };
        self.spaces.push(inside.then(offset));
        Ok(wxr::ReferenceSpace::new(
            base.kind,
            (self.spaces.len() - 1) as u32,
        ))
    }

    fn begin(&mut self, _now: Duration, out: &mut wxr::Frame) -> Result<(), wxr::Error> {
        self.release();
        out.views_mut().clear();
        out.state = wxr::FrameState::Wait;

        // The layer has a frame when it has one, and `nil` when it does not: paused, invalidated, or a frame
        // already in flight. Waiting is the right answer to all three, and none of them is an error.
        // SAFETY: the layer renderer is live, and a frame it returns is used until `end` gives it back.
        self.frame = unsafe { cp_layer_renderer_query_next_frame(&self.renderer) };
        if self.frame.is_null() {
            return Ok(());
        }

        // The compositor's order matters here: `cp_frame_predict_timing` must be called before the frame's
        // drawable is queried - the header says so - and a frame asked the other way round is a client bug the
        // compositor aborts on. The timing is what every pose in the frame is predicted for.
        //
        // SAFETY: the frame is the layer's, it is not in flight twice, and it is read between this
        // `start_update` and the `end_update` that `views` performs.
        unsafe {
            cp_frame::start_update(self.frame);
            let timing = cp_frame::predict_timing(self.frame);
            // `now` is not used: the compositor's clock is a Mach one and comparing it against a wall clock
            // is a comparison between two unrelated epochs. What it predicts is what is wanted anyway.
            let seconds =
                cp_time::to_cf_time_interval(cp_frame_timing::presentation_time(timing)).max(0.0);
            self.predicted = Duration::from_secs_f64(seconds);

            // Where the head will be when this is shown, predicted for the presentation time. It is kept
            // because the drawable that is told about it does not exist yet.
            self.origin = self.arkit.as_ref().and_then(|arkit| arkit.device(seconds));
        }

        // SAFETY: the frame is live, the timing above is already read, and the drawables are read before the
        // update window closes.
        self.drawable = unsafe {
            let array = cp_frame::query_drawables(self.frame);
            if array.is_null() {
                std::ptr::null_mut()
            } else {
                // The array holds one drawable per display target - what the wearer sees, and a capture one
                // for streaming - and it is the built-in one that a person is looking at.
                (0..cp_drawable_array::count(array))
                    .map(|index| cp_drawable_array::drawable(array, index))
                    .find(|drawable| {
                        !drawable.is_null()
                            && cp_drawable::target(*drawable) == cp_drawable_target::built_in
                    })
                    .unwrap_or(std::ptr::null_mut())
            }
        };
        if self.drawable.is_null() {
            // A frame with no drawable is discarded, but the update window it opened still has to close
            // before the frame is given back: `views` is not reached for a frame that is not rendered.
            // SAFETY: the frame is begun above and has not been given back.
            unsafe { cp_frame::end_update(self.frame) };
            self.release();
            return Ok(());
        }

        // SAFETY: the drawable is this frame's and is live until `end`.
        unsafe {
            // Where the head will be when this is shown, handed to the compositor, which compares it with
            // where the head actually is and reprojects the frame if the two disagree. Doing this is what
            // makes content hold still in the room instead of swimming behind every movement of the head.
            if let (Some(arkit), Some(_)) = (&self.arkit, self.origin) {
                sys::cp_drawable_set_device_anchor(self.drawable, arkit.anchor());
            }

            // Far first: a `CompositorServices` drawable wants reverse-Z, and its pair is read that way round
            // - see `sys::cp_drawable_set_depth_range`.
            if let Some((near, far)) = self.depth_range {
                sys::cp_drawable_set_depth_range(self.drawable, sys::Float2([far, near]));
            }
            for index in 0..cp_drawable::texture_count(self.drawable) {
                self.textures.push(FrameImage {
                    color: cp_drawable::color_texture(self.drawable, index),
                    depth: cp_drawable::depth_texture(self.drawable, index),
                });
            }
        }

        out.predicted_display_time = self.predicted;
        out.state = wxr::FrameState::Render;
        Ok(())
    }

    fn views(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        if self.drawable.is_null() {
            return Ok(());
        }
        // The space itself, and then the head in it: the eye transforms are already relative to the wearer, so
        // the viewer is the space's own origin and the eyes hang off it.
        let origin = self.space_origin(space);
        out.viewer = wxr_render::pose_from_transform(origin);

        // SAFETY: the drawable is this frame's and is live until `end`. Every index below is below the view
        // count just read, and the texture map lives as long as the view it came from.
        let (count, maps) = unsafe {
            let count = cp_drawable::view_count(self.drawable);
            let maps = (0..count)
                .map(|index| {
                    let view = cp_drawable::view(self.drawable, index);
                    let map = cp_view::view_texture_map(view);
                    (
                        // `device from view`: the eye's own place in device space. It is a pose already, so
                        // nothing inverts it - the world-from-eye transform is `origin * this`.
                        Mat4::from_cols_array(&sys::cp_view_get_transform(view).0),
                        // A mixed-reality layer refuses `cp_view_get_tangents` and wants the projection
                        // matrix instead; the four openings are read back out of it for `fov` below.
                        Mat4::from_cols_array(
                            &sys::cp_drawable_compute_projection(
                                self.drawable,
                                cp_axis_direction_convention::right_up_back,
                                index,
                            )
                            .0,
                        ),
                        cp_view_texture_map::texture_index(map),
                        cp_view_texture_map::slice_index(map) as u32,
                        cp_view_texture_map::viewport(map),
                    )
                })
                .collect::<Vec<_>>();
            (count, maps)
        };

        for (index, (device_from_eye, projection, image, layer, viewport)) in
            maps.into_iter().enumerate()
        {
            out.views_mut().push(wxr::View {
                eye: if count == 1 {
                    wxr::Eye::Mono
                } else if index == 0 {
                    wxr::Eye::Left
                } else {
                    wxr::Eye::Right
                },
                pose: wxr_render::pose_from_transform(origin * device_from_eye),
                fov: wxr_render::angles(projection),
                viewport: wxr::Viewport {
                    x: viewport.originX.max(0.0) as u32,
                    y: viewport.originY.max(0.0) as u32,
                    width: viewport.width.max(0.0) as u32,
                    height: viewport.height.max(0.0) as u32,
                },
                image,
                layer,
                recommended_viewport_scale: None,
            });
        }

        // The queries are finished; rendering starts after this and the frame is submitted in `end`.
        // SAFETY: the frame is the one begun above and has not been given back.
        unsafe { cp_frame::end_update(self.frame) };
        Ok(())
    }

    fn inputs(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::InputSource>,
    ) -> Result<(), wxr::Error> {
        let Some(arkit) = &self.arkit else {
            return Ok(());
        };
        // The same space the views are expressed in, for the same reason.
        let origin = self.space_origin(space);

        // Hands are picked up and put down, and that is the set of inputs changing - one event however many
        // hands it is about, which is the shape WebXR gives `inputsourceschange` too.
        let hands = arkit.hands();
        if hands.len() != self.hands_seen {
            self.hands_seen = hands.len();
            self.inputs_changed = true;
        }

        for (index, (handedness, transform, tracked)) in hands.into_iter().enumerate() {
            let pose = wxr_render::pose_from_transform(origin * transform);
            out.push(wxr::InputSource {
                id: wxr::InputId::new(index as u32),
                handedness,
                // A palm ray is a tracked pointer like any other; the C surface has no gaze or screen ray to
                // make.
                target_ray_mode: wxr::TargetRayMode::TrackedPointer,
                // No skeleton, and that is the C surface rather than a gap somebody chose: ARKit's hand anchors
                // are a palm pose, and the joints are in the Swift `HandAnchor.skeleton` this crate cannot see.
                // So there is nothing to ask for and `Session::hand` would have nothing to answer with.
                hand: false,
                // A hand here is a place and an orientation, and that is all the C API gives - the skeleton
                // is the Swift API's, so there is no fingertip to aim from and no pinch to read. Grip and
                // aim are therefore the same pose, and the pose's own orientation is the palm's direction:
                // a game that wants a ray takes its -Z, which is what every ray in this workspace is.
                grip: pose,
                aim: pose,
                tracked,
                // And no buttons at all, which is not a gap: a hand has none. The core's buttons are the
                // intersection of the three platforms, and this is the platform where the intersection is
                // empty - so there are no press events here either, because there is nothing to press.
                buttons: wxr::Buttons::default(),
                axes: wxr::Axes::default(),
            });
        }
        Ok(())
    }

    fn planes(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Plane>,
    ) -> Result<(), wxr::Error> {
        let Some(arkit) = &self.arkit else {
            return Ok(());
        };
        // ARKit reports planes in the session's own origin, which is this backend's `Local`; a caller
        // asking for another space gets them brought over, the same way the views are.
        let origin = self.space_origin(space);
        let (_, orientation, position) = origin.to_scale_rotation_translation();
        let origin = wxr::Pose {
            position,
            orientation,
        };
        out.extend(arkit.planes().into_iter().map(|mut plane| {
            plane.pose = plane.pose.relative_to(origin);
            plane
        }));
        Ok(())
    }

    fn anchor(
        &mut self,
        space: wxr::ReferenceSpace,
        pose: wxr::Pose,
    ) -> Result<wxr::Anchor, wxr::Error> {
        // The anchor's place is asked for in `space`; ARKit takes one in the session's origin, which this
        // backend's `Local` is - so the space's origin is composed in front, the way a view is.
        let world_from_anchor = self.space_origin(space) * Mat4::from(pose.transform());
        self.arkit
            .as_mut()
            .and_then(|arkit| arkit.add_anchor(world_from_anchor))
            .ok_or_else(|| wxr::Error::Unsupported("anchors".into()))
    }

    fn anchor_pose(
        &mut self,
        anchor: wxr::Anchor,
        space: wxr::ReferenceSpace,
    ) -> Result<Option<wxr::Pose>, wxr::Error> {
        let Some(arkit) = &self.arkit else {
            return Ok(None);
        };
        let Some(world) = arkit.anchor_pose(anchor) else {
            return Ok(None);
        };
        let (_, orientation, position) = world.to_scale_rotation_translation();
        let origin = self.space_origin(space);
        let (_, origin_orientation, origin_position) = origin.to_scale_rotation_translation();
        Ok(Some(
            wxr::Pose {
                position,
                orientation,
            }
            .relative_to(wxr::Pose {
                position: origin_position,
                orientation: origin_orientation,
            }),
        ))
    }

    fn release_anchor(&mut self, anchor: wxr::Anchor) {
        if let Some(arkit) = self.arkit.as_mut() {
            arkit.release_anchor(anchor);
        }
    }

    fn end(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        if self.frame.is_null() {
            return Ok(());
        }
        // SAFETY: the frame is the one begun above and has not been given back.
        unsafe {
            cp_frame::start_submission(self.frame);
            self.present();
            cp_frame::end_submission(self.frame);
        }
        self.release();
        Ok(())
    }
}

/// A Metal pixel format in the core's terms.
///
/// `Unknown` for a format this core has not learned, which the renderer turns into a frame it does not draw
/// - better than one drawn through the wrong answer about what the bits mean.
fn color_format(format: MTLPixelFormat) -> wxr::ColorFormat {
    match format {
        MTLPixelFormat::BGRA8Unorm_sRGB => wxr::ColorFormat::Bgra8Srgb,
        MTLPixelFormat::BGRA8Unorm => wxr::ColorFormat::Bgra8Unorm,
        MTLPixelFormat::RGBA8Unorm_sRGB => wxr::ColorFormat::Rgba8Srgb,
        MTLPixelFormat::RGBA8Unorm => wxr::ColorFormat::Rgba8Unorm,
        MTLPixelFormat::RGBA16Float => wxr::ColorFormat::Rgba16Float,
        MTLPixelFormat::RGB10A2Unorm => wxr::ColorFormat::Rgb10a2Unorm,
        _ => wxr::ColorFormat::Unknown,
    }
}
