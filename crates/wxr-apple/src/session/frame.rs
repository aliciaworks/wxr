//! The frame lifecycle: taking the compositor's frame, reading the views off it, and handing it back.
//!
//! Separate from `dispatch` for the same reason `sources` is, and it is the more complete case: the
//! `wxr::Session` implementation is one block of *answers*, and this is the part of it that is a sequence -
//! a frame is asked for, its drawables are chosen, its views are read inside a window the compositor opens,
//! and it is submitted. `dispatch` is read for what this session says; this is read for what it does, in
//! what order, and which of those steps the compositor is holding a door open for.
//!
//! **The ordering is the content.** `predict_timing` before the drawable is queried, the drawable read
//! inside the update window, `end_update` in the branch that renders and *not* in the one where the frame
//! was cancelled. None of that is this backend's policy - it is what the `CompositorServices` headers say
//! happens to a client that does otherwise, and a frame that is submitted out of order is a frame the
//! compositor aborts on rather than one it draws badly.

use super::*;

/// One eye as the drawable describes it: the eye's place in device space, its projection, and the part of
/// the frame's images it draws into.
///
/// Cached because the compositor's update window closes on the first `views`, and reading the drawable after
/// that is the access its header forbids. A caller that asks for the same frame's views again - in another
/// space, or because it retried - gets this frame's eyes with the new head, not a second trip to the drawable.
pub(super) struct EyeView {
    device_from_eye: Mat4,
    projection: Mat4,
    image: usize,
    layer: u32,
    viewport: wxr::Viewport,
}

impl AppleSession {
    pub(super) fn begin_impl(
        &mut self,
        _now: Duration,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        self.release();
        out.reset();
        // An invalidated layer hands out no more frames, and waiting for one is a frame loop that never stops.
        // The state is already `Ended`, so this is the answer the core has for it rather than a second guess.
        if self.layer_state().0 == wxr::State::Ended {
            out.state = wxr::FrameState::Exit;
            return Ok(());
        }
        out.state = wxr::FrameState::Wait;

        // The layer has a frame when it has one, and `nil` when it does not: paused, or a frame already in
        // flight. Waiting is the right answer to both, and neither of them is an error.
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
        let (queried, drawable) = unsafe {
            let array = cp_frame::query_drawables(self.frame);
            if array.is_null() {
                (0, std::ptr::null_mut())
            } else {
                // The array holds one drawable per display target - what the wearer sees, and a capture one
                // for streaming - and it is the built-in one that a person is looking at.
                let count = cp_drawable_array::count(array);
                let found = (0..count)
                    .map(|index| cp_drawable_array::drawable(array, index))
                    .find(|drawable| {
                        !drawable.is_null()
                            && cp_drawable::target(*drawable) == cp_drawable_target::built_in
                    })
                    .unwrap_or(std::ptr::null_mut());
                (count, found)
            }
        };
        self.drawable = drawable;
        if queried == 0 {
            // The header on `cp_frame_query_drawables` is exact about this case: an empty array means the
            // frame was cancelled, and it "should be discarded and is invalid to access" - which is what
            // makes the `end_update` in the branch below wrong here rather than merely unnecessary. The
            // update window opened above is left to the compositor: it cancelled the frame, so there is
            // nothing to close it on, and closing it is the access the header forbids. This is a cold
            // start's answer - the immersive space is still coming up and the layer hands out a frame it
            // will not serve - so the frame goes back untouched and the next one is asked for.
            self.release();
            return Ok(());
        }
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

    pub(super) fn views_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        if !self.has_space(space) {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        // Asking twice replaces the eyes rather than adding a second pair to the same frame.
        out.views_mut().clear();
        if self.drawable.is_null() {
            return Ok(());
        }
        // The compositor's update window is open until the first `views` closes it, and the drawable may not
        // be read afterwards. Read its eyes once, keep them, and close the window here rather than on every
        // call - a second call uses the cache.
        if !self.updated {
            // SAFETY: the drawable is this frame's and is live until `end`. Every index below is below the
            // view count just read, and the texture map lives as long as the view it came from.
            self.eyes = unsafe {
                (0..cp_drawable::view_count(self.drawable))
                    .map(|index| {
                        let view = cp_drawable::view(self.drawable, index);
                        let map = cp_view::view_texture_map(view);
                        let viewport = cp_view_texture_map::viewport(map);
                        EyeView {
                            // `device from view`: the eye's own place in device space. It is a pose
                            // already, so nothing inverts it - the eye-in-space transform is `head * this`.
                            device_from_eye: Mat4::from_cols_array(
                                &sys::cp_view_get_transform(view).0,
                            ),
                            // A mixed-reality layer refuses `cp_view_get_tangents` and wants the projection
                            // matrix instead; the four openings are read back out of it for `fov` below.
                            projection: Mat4::from_cols_array(
                                &sys::cp_drawable_compute_projection(
                                    self.drawable,
                                    cp_axis_direction_convention::right_up_back,
                                    index,
                                )
                                .0,
                            ),
                            image: cp_view_texture_map::texture_index(map),
                            layer: cp_view_texture_map::slice_index(map) as u32,
                            viewport: wxr::Viewport {
                                x: viewport.originX.max(0.0) as u32,
                                y: viewport.originY.max(0.0) as u32,
                                width: viewport.width.max(0.0) as u32,
                                height: viewport.height.max(0.0) as u32,
                            },
                        }
                    })
                    .collect::<Vec<_>>()
            };
            // The queries are finished; rendering starts after this and the frame is submitted in `end`.
            // SAFETY: the frame is the one begun above and has not been given back.
            unsafe { cp_frame::end_update(self.frame) };
            self.updated = true;
        }

        // The head in the space, and then each eye around it: the compositor's eye transforms are already
        // relative to the wearer, so the viewer is the device and an eye is `head * device_from_eye`.
        let head = self.device_in_space(space);
        out.viewer = wxr_render::pose_from_transform(head);
        let count = self.eyes.len();
        for (index, eye) in self.eyes.iter().enumerate() {
            out.views_mut().push(wxr::View {
                eye: if count == 1 {
                    wxr::Eye::Mono
                } else if index == 0 {
                    wxr::Eye::Left
                } else {
                    wxr::Eye::Right
                },
                pose: wxr_render::pose_from_transform(head * eye.device_from_eye),
                fov: wxr_render::angles(eye.projection),
                viewport: eye.viewport,
                image: eye.image,
                layer: eye.layer,
                recommended_viewport_scale: None,
            });
        }
        Ok(())
    }

    pub(super) fn end_impl(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        if self.frame.is_null() {
            return Ok(());
        }
        // A caller may end a frame without asking for its views - one that decided not to draw, or that
        // only wanted the timing `begin` predicted. The update window is still open then, and submission
        // has to follow its close rather than open it. `views` is the normal place this happens, because
        // the caller renders between the two; a frame with no views has nothing to render.
        // SAFETY: the frame is the one begun above and has not been given back.
        if !self.updated {
            unsafe { cp_frame::end_update(self.frame) };
            self.updated = true;
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
