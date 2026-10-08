//! The frame loop, as this backend answers it: what the runtime is doing, and the frame that comes out
//! of asking.
//!
//! `wxr::Session` gets one implementation per type and this one is six hundred lines, so each answer lives
//! here and the trait is the list of what is answered - a delegation and the reason for it in one place.

use super::*;

impl OpenXrSession {
    pub(super) fn begin_impl(
        &mut self,
        _now: Duration,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        // A runtime is not asked to wait for a frame until the session has been begun. `xrWaitFrame` before
        // `xrBeginSession` is an error at best, and the event that says to begin arrives on the same poll the
        // caller is not making while it waits - so waiting on it is a deadlock rather than a frame. A session
        // that has not begun is a frame to wait for.
        if !self.begun {
            out.views_mut().clear();
            out.state = wxr::FrameState::Wait;
            return Ok(());
        }

        // The runtime is asked to wait, and it answers with the frame's timing: when the picture will be
        // shown, and whether there is anything to draw at all. `now` is not used - OpenXR's clock is the
        // runtime's, and comparing a wall clock against it is a comparison between two unrelated epochs.
        let state = self
            .waiter
            .wait()
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        self.predicted = state.predicted_display_time;

        out.state = if state.should_render {
            wxr::FrameState::Render
        } else {
            wxr::FrameState::Wait
        };
        out.predicted_display_time =
            Duration::from_nanos(state.predicted_display_time.as_nanos().max(0) as u64);
        out.views_mut().clear();

        if out.state != wxr::FrameState::Render {
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
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;

        let held = self
            .swapchain
            .acquire_image()
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        // Unbounded: the runtime is the one that knows when the image is free, and a timeout here would be
        // this code deciding it knows better.
        self.swapchain
            .wait_image(xr::Duration::INFINITE)
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        self.held = Some(held);

        // Each layer has a swapchain of its own, and each is taken in the same frame the eyes are: an image the
        // runtime has not handed over is not one an app may draw into, and waiting is unbounded for the reason
        // the eyes' wait is - the runtime is the one that knows when it is free.
        for layer in self.layers.iter_mut().flatten() {
            let held = layer
                .swapchain
                .acquire_image()
                .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
            layer
                .swapchain
                .wait_image(xr::Duration::INFINITE)
                .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
            layer.held = Some(held);
        }
        Ok(())
    }
}

impl OpenXrSession {
    pub(super) fn views_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        let Some(reference) = self.spaces.get(space.id() as usize) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let (_, located) = self
            .session
            .locate_views(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                self.predicted,
                reference,
            )
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        if located.len() != 2 {
            return Err(wxr::Error::Present(
                "the runtime located an odd number of views".into(),
            ));
        }

        // The head, in the same space the views are in - the eyes are placed around it, and a scene that wants
        // the camera on the wearer rather than on an eye asks for it.
        if let Ok(location) = self.view.locate(reference, self.predicted) {
            out.viewer = pose(location.pose);
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
                recommended_viewport_scale: None,
            });
        }
        self.located = located;
        Ok(())
    }
}

impl OpenXrSession {
    pub(super) fn end_impl(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        self.held = None;
        self.swapchain
            .release_image()
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        for layer in self.layers.iter_mut().flatten() {
            if layer.held.take().is_some() {
                layer
                    .swapchain
                    .release_image()
                    .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
            }
        }

        // The layer borrows this frame's views, so it is built here and lives only until it is submitted.
        // Each eye is a rectangle of the one image, in its own array layer.
        let extent = xr::Extent2Di {
            width: self.extent.width as i32,
            height: self.extent.height as i32,
        };
        let views: Vec<_> = self
            .located
            .iter()
            .enumerate()
            .map(|(index, view)| {
                xr::CompositionLayerProjectionView::new()
                    .pose(view.pose)
                    .fov(view.fov)
                    .sub_image(
                        xr::SwapchainSubImage::new()
                            .swapchain(&self.swapchain)
                            .image_array_index(index as u32)
                            .image_rect(xr::Rect2Di {
                                offset: xr::Offset2Di { x: 0, y: 0 },
                                extent,
                            }),
                    )
            })
            .collect();
        let layer = xr::CompositionLayerProjection::new()
            .space(&self.spaces[0])
            .views(&views);

        // The app's layers, in the order they were made: a quad is a rectangle at a pose, and the whole of its
        // swapchain image is the picture. They are built here and not kept, because a sub-image borrows the
        // swapchain and this is the one place a frame is submitted.
        let quads: Vec<xr::CompositionLayerQuad<'_, xr::Vulkan>> = self
            .layers
            .iter()
            .flatten()
            .filter_map(|layer| {
                Some(
                    xr::CompositionLayerQuad::new()
                        // A layer is a picture *over* what is already there, so its alpha is part of the
                        // picture: without this bit the compositor ignores the channel and a panel with a
                        // transparent background is a black rectangle. Opaque content says the same thing with
                        // every alpha at one.
                        .layer_flags(xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA)
                        .space(self.spaces.get(layer.space)?)
                        .eye_visibility(xr::EyeVisibility::BOTH)
                        .pose(layer.pose)
                        .size(layer.size)
                        .sub_image(
                            xr::SwapchainSubImage::new()
                                .swapchain(&layer.swapchain)
                                .image_array_index(0)
                                .image_rect(xr::Rect2Di {
                                    offset: xr::Offset2Di { x: 0, y: 0 },
                                    extent: xr::Extent2Di {
                                        width: layer.extent.width as i32,
                                        height: layer.extent.height as i32,
                                    },
                                }),
                        ),
                )
            })
            .collect();

        // The world first and the app's pictures over it: the projection layer is what a scene is drawn into,
        // and a panel that arrived behind it would be a panel nobody sees.
        let mut present: Vec<&xr::CompositionLayerBase<'_, xr::Vulkan>> =
            Vec::with_capacity(1 + quads.len());
        present.push(&layer);
        for quad in &quads {
            present.push(quad);
        }

        self.stream
            .end(self.predicted, self.blend, &present)
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        Ok(())
    }
}

impl OpenXrSession {
    pub(super) fn poll_impl(&mut self) -> Option<wxr::Event> {
        // The press edges the last frame produced come first: they are this crate's own, and waiting for the
        // runtime to get round to an instance event to report them would be a delay with no cause.
        if let Some(hands) = self.hands.as_mut()
            && let Some(event) = hands.poll()
        {
            return Some(event);
        }

        // Events come off the instance, one at a time. A state change is the one a frame loop acts on and a
        // profile change is the set of inputs changing under it; the rest are the runtime's business, and
        // skipping them is better than inventing a mapping for them.
        loop {
            match self.instance.poll_event(&mut self.events) {
                // Which controllers the runtime has bound, which is the set of inputs changing: what they then
                // *are* is the next frame's answer, from `inputs`.
                Ok(Some(xr::Event::InteractionProfileChanged(_))) => {
                    return Some(wxr::Event::InputsChanged);
                }
                Ok(Some(xr::Event::SessionStateChanged(event))) => {
                    let openxr = event.state();
                    self.openxr_state = openxr;
                    // OpenXR's ladder is where both of the core's axes are read from, and the interesting
                    // rung is `VISIBLE` without `FOCUSED`: a session on a display that nobody is attending
                    // to, which is exactly WebXR's `visible-blurred`.
                    // `STOPPING` is the runtime asking the app to end the session rather than the session
                    // already being over: the one thing `ExitRequested` is for, and the app is expected to
                    // shut down cleanly rather than have it done for it. The session is still here, so it is
                    // still `Ready` - just not on a display any more.
                    if openxr == xr::SessionState::STOPPING {
                        self.state = wxr::State::Ready;
                        self.visibility = wxr::Visibility::Hidden;
                        return Some(wxr::Event::ExitRequested);
                    }
                    let (state, visibility) = match openxr {
                        xr::SessionState::READY => {
                            // A session runs only after the app has begun it, and only once it is ready -
                            // the runtime says `XR_ERROR_SESSION_NOT_RUNNING` from `xrWaitFrame` until then,
                            // which is the one thing it will not do on the app's behalf.
                            if !self.begun {
                                if let Err(error) = self
                                    .session
                                    .begin(xr::ViewConfigurationType::PRIMARY_STEREO)
                                {
                                    log::error!("wxr-openxr: beginning the session: {error:?}");
                                }
                                self.begun = true;
                            }
                            (wxr::State::Ready, wxr::Visibility::Hidden)
                        }
                        xr::SessionState::IDLE | xr::SessionState::SYNCHRONIZED => {
                            (wxr::State::Ready, wxr::Visibility::Hidden)
                        }
                        xr::SessionState::VISIBLE => {
                            (wxr::State::Ready, wxr::Visibility::VisibleBlurred)
                        }
                        xr::SessionState::FOCUSED => (wxr::State::Ready, wxr::Visibility::Visible),
                        xr::SessionState::LOSS_PENDING | xr::SessionState::EXITING => {
                            (wxr::State::Ended, wxr::Visibility::Hidden)
                        }
                        // A state this crate has not learned is not a state to guess at: the session is
                        // still whatever it was, and the next event will say what happened.
                        _ => continue,
                    };
                    if state != self.state {
                        self.state = state;
                        return Some(wxr::Event::StateChanged(state));
                    }
                    if visibility != self.visibility {
                        self.visibility = visibility;
                        return Some(wxr::Event::VisibilityChanged(visibility));
                    }
                    continue;
                }
                Ok(Some(_)) => continue,
                Ok(None) => {
                    // A session the runtime has taken away is `Lost` once, after the state that said so: a frame
                    // loop that keeps polling is told the difference between over and gone.
                    if self.state == wxr::State::Ended && !self.lost {
                        self.lost = true;
                        return Some(wxr::Event::Lost);
                    }
                    return None;
                }
                Err(error) => {
                    log::error!("wxr-openxr: polling events: {error:?}");
                    self.state = wxr::State::Ended;
                    self.lost = true;
                    return Some(wxr::Event::Lost);
                }
            }
        }
    }
}
