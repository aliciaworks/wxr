//! The frame loop, as this backend answers it: what the runtime is doing, where the eyes are, and which
//! images the frame is holding.
//!
//! `wxr::Session` gets one implementation per type and this one is six hundred lines, so each answer lives
//! in a file of its own and the trait is the list of what is answered - a delegation and the reason for it
//! in one place. Closing the frame is `submit`, which is the other half of this and needs the images here.

use super::*;

impl OpenXrSession {
    pub(super) fn begin_impl(
        &mut self,
        _now: Duration,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        // Last frame's eyes and last frame's head are not this frame's, on every path out of here.
        out.reset();
        // Nor is last frame's measurement this frame's. A successful `end` clears it too, but an `end` that
        // failed after taking the depth image - or a `Wait` frame - would otherwise leave it for `depth` to
        // answer with, and the runtime only allows one acquire per frame.
        self.measured = None;
        self.depth_asked = false;
        // A session the runtime has taken away has no frame to wait for. `xrWaitFrame` on one is an error,
        // and a caller that keeps asking is a loop that ends by error rather than by the `Exit` the core has.
        if self.state == wxr::State::Ended {
            out.state = wxr::FrameState::Exit;
            return Ok(());
        }
        // A runtime is not asked to wait for a frame until the session has been begun. `xrBeginSession` is
        // legal only once the runtime has said `READY`, which is news `poll` carries, so a session whose
        // caller has not polled yet is a frame to wait for rather than a deadlock.
        //
        // The begin is here and not in `poll` because it is part of the frame that asks for one: a failure
        // is this call's error, and a session that failed to begin is still one to try again rather than a
        // session marked begun that every `xrWaitFrame` will then refuse.
        if !self.begun {
            if self.openxr_state != xr::SessionState::READY {
                out.state = wxr::FrameState::Wait;
                return Ok(());
            }
            self.session
                .begin(xr::ViewConfigurationType::PRIMARY_STEREO)
                .map_err(|error| {
                    wxr::Error::Present(format!("starting the session: {error:?}").into())
                })?;
            self.begun = true;
        }

        // The runtime is asked to wait, and it answers with the frame's timing: when the picture will be
        // shown, and whether there is anything to draw at all. `now` is not used - OpenXR's clock is the
        // runtime's, and comparing a wall clock against it is a comparison between two unrelated epochs.
        let state = self
            .waiter
            .wait()
            .map_err(|error| wxr::Error::Present(format!("{error:?}").into()))?;
        self.predicted = state.predicted_display_time;

        out.state = if state.should_render {
            wxr::FrameState::Render
        } else {
            wxr::FrameState::Wait
        };
        out.predicted_display_time =
            Duration::from_nanos(state.predicted_display_time.as_nanos().max(0) as u64);
        // A frame the app never asks views of must not submit the views the last frame located, nor a
        // projection layer in the space that frame used: both are about an image that is already gone.
        self.located.clear();
        self.view_space = None;

        if out.state != wxr::FrameState::Render {
            // The runtime asked for a frame it does not want drawn, and OpenXR's frame loop is still a loop:
            // one `xrWaitFrame` is one `xrBeginFrame` and one `xrEndFrame`, and a frame with nothing in it is a
            // frame submitted with no layers at all. Closing the pair here - the caller is told `Wait` and has
            // nothing to draw, so it never reaches `Session::end` - is what keeps the *next* `xrWaitFrame`
            // from waiting for a frame that is never coming.
            self.stream
                .begin()
                .map_err(|error| wxr::Error::Present(format!("{error:?}").into()))?;
            self.stream
                .end(self.predicted, self.blend, &[])
                .map_err(|error| wxr::Error::Present(format!("{error:?}").into()))?;
            return Ok(());
        }

        // Once a frame and before anything is read: OpenXR resolves the bindings here, and a pose read
        // before this is the pose from the frame before.
        if let Some(hands) = &self.hands
            && let Err(error) = hands.sync(&self.session)
        {
            log::debug!("wxr-openxr: syncing the actions: {error}");
        }

        self.stream
            .begin()
            .map_err(|error| wxr::Error::Present(format!("{error:?}").into()))?;

        let held = match self.swapchain.acquire_image() {
            Ok(held) => held,
            Err(error) => {
                self.abandon_frame();
                return Err(wxr::Error::Present(format!("{error:?}").into()));
            }
        };
        // Unbounded: the runtime is the one that knows when the image is free, and a timeout here would be
        // this code deciding it knows better.
        if let Err(error) = self.swapchain.wait_image(xr::Duration::INFINITE) {
            // The image was acquired but never handed over, so there is nothing here that may be released.
            // The frame itself still has to close, or every later `xrBeginFrame` is a call-order error.
            self.abandon_frame();
            return Err(wxr::Error::Present(format!("{error:?}").into()));
        }
        self.held = Some(held);
        // The depth image is taken in the same frame and the same order: a colour image the app may draw into
        // beside a depth image it may not is no frame at all, and the two are one pass.
        if let Err(error) = self.acquire_target_depth() {
            self.abandon_frame();
            return Err(error);
        }
        // The motion vectors are the other half of space warp and are taken in the same frame as the depth:
        // one without the other is a synthesised frame with nothing to synthesise from, so the pair is the
        // unit here as well.
        if self.space_warp
            && let Err(error) = self.acquire_target_motion()
        {
            self.abandon_frame();
            return Err(error);
        }
        // Both images are now this frame's, which is the pair `image` hands the renderer.
        self.record_frame_images();

        // Each layer has a swapchain of its own, and each is taken in the same frame the eyes are: an image the
        // runtime has not handed over is not one an app may draw into, and waiting is unbounded for the reason
        // the eyes' wait is - the runtime is the one that knows when it is free.
        for index in 0..self.layers.len() {
            if let Err(error) = self.acquire_layer(index) {
                self.abandon_frame();
                return Err(error);
            }
        }
        Ok(())
    }

    /// Take one app layer's image for this frame, if that layer still exists.
    ///
    /// Separate from the loop so a failure ends the layer borrow before the caller has to close a frame that
    /// is holding the images already taken.
    fn acquire_layer(&mut self, index: usize) -> Result<(), wxr::Error> {
        let Some(layer) = self.layers[index].as_mut() else {
            return Ok(());
        };
        let held = layer
            .swapchain
            .acquire_image()
            .map_err(|error| wxr::Error::Present(format!("{error:?}").into()))?;
        layer
            .swapchain
            .wait_image(xr::Duration::INFINITE)
            .map_err(|error| wxr::Error::Present(format!("{error:?}").into()))?;
        layer.held = Some(held);
        Ok(())
    }

    /// Give back a frame whose images could not all be taken.
    ///
    /// Once `xrBeginFrame` has succeeded the pair has to close, even when nothing was drawn: an open frame is
    /// every later `xrBeginFrame` failing. An image already waited for is released first, because the runtime
    /// will not hand it out again until it is. An image whose wait failed is left alone - the binding refuses
    /// to release one it never waited for - and the runtime has already lost that frame.
    fn abandon_frame(&mut self) {
        if self.held.take().is_some() {
            let _ = self.swapchain.release_image();
        }
        let _ = self.release_target_depth();
        let _ = self.release_target_motion();
        for layer in self.layers.iter_mut().flatten() {
            if layer.held.take().is_some() {
                let _ = layer.swapchain.release_image();
            }
        }
        let _ = self.stream.end(self.predicted, self.blend, &[]);
    }
}

impl OpenXrSession {
    pub(super) fn views_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        // Asking for views twice in one frame replaces them; it does not give the frame four eyes, nor a head
        // from the first answer over no eyes. The frame's whole answer goes before anything here can fail, and
        // with it the runtime's own views and the space they were located in - so a second call that fails
        // cannot leave `end` submitting the first call's eyes either.
        out.reset();
        self.located.clear();
        self.view_space = None;
        let Some(index) = self.space_index(space) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let reference = &self.spaces[index];
        let (state, located) = self
            .session
            .locate_views(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                self.predicted,
                reference,
            )
            .map_err(|error| wxr::Error::Present(format!("{error:?}").into()))?;
        // A view is a place and a direction, and `xrLocateViews` reports the two halves' validity
        // separately. The Rust binding zeroes an invalid half, so reading a view anyway would put an eye at
        // the origin with an identity orientation rather than leaving the frame without one. Both valid is a
        // full pose; anything less is a frame the runtime cannot say where the eyes are, which is a frame
        // with no views - and `end` then submits no projection layer rather than one at a made-up pose.
        if !state.contains(xr::ViewStateFlags::POSITION_VALID)
            || !state.contains(xr::ViewStateFlags::ORIENTATION_VALID)
        {
            return Ok(());
        }
        if located.len() != 2 {
            return Err(wxr::Error::Present(
                "the runtime located an odd number of views".into(),
            ));
        }

        // The head, in the same space the views are in - the eyes are placed around it, and a scene that wants
        // the camera on the wearer rather than on an eye asks for it. It is checked the same way: a runtime
        // that will not stand behind the head's pose leaves the frame's viewer at its reset identity.
        // The head's velocity arrives beside its location, from the same `xrLocateSpace` - which is the call
        // a reprojecting renderer is making anyway, so the speed costs it nothing.
        if let Ok((location, velocity)) = self.view.relate(reference, self.predicted)
            && is_located(location.location_flags)
        {
            out.viewer = pose(location.pose);
            out.motion = crate::motion(&velocity);
            // OpenXR separates "valid" from "tracked": a position that is valid but not tracked is the
            // runtime's estimate, which is WebXR's `emulatedPosition` by another name.
            out.emulated_position = !location
                .location_flags
                .contains(xr::SpaceLocationFlags::POSITION_TRACKED);
        }

        let image = self.held.unwrap_or(0);
        let views = out.views_mut();
        for (index, view) in located.iter().enumerate() {
            views.push(wxr::View {
                eye: if index == 0 {
                    wxr::Eye::Left
                } else {
                    wxr::Eye::Right
                },
                pose: pose(view.pose),
                fov: field_of_view(view.fov),
                viewport: wxr::Viewport {
                    x: 0,
                    y: 0,
                    width: self.extent.width,
                    height: self.extent.height,
                },
                image: image as usize,
                layer: index as u32,
                // The two eyes of the headset, and no third camera: OpenXR locates the views the
                // view configuration names, and a stereo session's are the wearer's.
                is_first_person_observer: false,
                recommended_viewport_scale: None,
            });
        }
        // The layer is submitted after this, and the runtime reads the poses above relative to the space
        // they were located in - which is not always the first space the app made.
        self.located = located;
        self.view_space = Some(space.id() as usize);
        Ok(())
    }
}
