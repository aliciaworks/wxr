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
        // Last frame's eyes and last frame's head are not this frame's, on every path out of here.
        out.reset();
        // Nor is last frame's measurement this frame's. A successful `end` clears it too, but an `end` that
        // failed after taking the depth image - or a `Wait` frame - would otherwise leave it for `depth` to
        // answer with, and the runtime only allows one acquire per frame.
        self.measured = None;
        // A runtime is not asked to wait for a frame until the session has been begun. `xrWaitFrame` before
        // `xrBeginSession` is an error at best, and the event that says to begin arrives on the same poll the
        // caller is not making while it waits - so waiting on it is a deadlock rather than a frame. A session
        // that has not begun is a frame to wait for.
        if !self.begun {
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
                .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
            self.stream
                .end(self.predicted, self.blend, &[])
                .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
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
        let (state, located) = self
            .session
            .locate_views(
                xr::ViewConfigurationType::PRIMARY_STEREO,
                self.predicted,
                reference,
            )
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
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
        if let Ok(location) = self.view.locate(reference, self.predicted)
            && is_located(location.location_flags)
        {
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
        // The layer is submitted after this, and the runtime reads the poses above relative to the space
        // they were located in - which is not always the first space the app made.
        self.located = located;
        self.view_space = Some(space.id() as usize);
        Ok(())
    }
}

impl OpenXrSession {
    pub(super) fn end_impl(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        // A frame the runtime did not want drawn was already opened and closed by `begin_impl`, because the
        // caller was told `Wait` and has nothing to hand back. Ending it again is a call-order error at the
        // runtime - an image released that was never acquired - and this is the end of a frame that was never
        // begun, which is nothing at all.
        if self.held.is_none() {
            return Ok(());
        }

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
        // The same space the eyes were located in. The first space an app made was not necessarily the one
        // it asked for its views in, and a projection layer whose space disagrees with its view poses is a
        // scene at the wrong place - which is invisible until someone offsets a space and then very visible.
        //
        // A frame with no views has no projection layer at all: a projection layer is required to have a
        // view, and an app that submitted one with none is a validation error rather than a black frame.
        let projection = (!views.is_empty()).then(|| {
            let space = self
                .view_space
                .and_then(|id| self.spaces.get(id))
                .or_else(|| self.spaces.first())
                .unwrap_or(&self.view);
            xr::CompositionLayerProjection::new()
                .space(space)
                .views(&views)
        });

        // The app's layers, in the order they were made. Which struct one becomes is the extension it came
        // from: a quad is a rectangle at a pose, a cylinder is wrapped around one, an equirect is a sphere, and
        // a cube is a picture the app drew onto six faces. They are all built here and not kept, because a
        // sub-image borrows the swapchain and this is the one place a frame is submitted.
        let built: Vec<Built<'_>> = self
            .layers
            .iter()
            .flatten()
            .filter_map(|layer| {
                let space = self.spaces.get(layer.space)?;
                let flags = xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA;
                let rect = xr::Rect2Di {
                    offset: xr::Offset2Di { x: 0, y: 0 },
                    extent: xr::Extent2Di {
                        width: layer.extent.width as i32,
                        height: layer.extent.height as i32,
                    },
                };
                let sub_image = || {
                    xr::SwapchainSubImage::new()
                        .swapchain(&layer.swapchain)
                        .image_array_index(0)
                        .image_rect(rect)
                };
                Some(match layer.shape {
                    wxr::LayerShape::Quad { width, height } => Built::Quad(
                        xr::CompositionLayerQuad::new()
                            .layer_flags(flags)
                            .space(space)
                            .eye_visibility(xr::EyeVisibility::BOTH)
                            .pose(layer.pose)
                            .size(xr::Extent2Df { width, height })
                            .sub_image(sub_image()),
                    ),
                    wxr::LayerShape::Cylinder {
                        radius,
                        central_angle,
                        aspect,
                    } => Built::Cylinder(
                        xr::CompositionLayerCylinderKHR::new()
                            .layer_flags(flags)
                            .space(space)
                            .eye_visibility(xr::EyeVisibility::BOTH)
                            .pose(layer.pose)
                            .radius(radius)
                            .central_angle(central_angle)
                            .aspect_ratio(aspect)
                            .sub_image(sub_image()),
                    ),
                    wxr::LayerShape::Equirect {
                        radius,
                        central_horizontal,
                        upper_vertical,
                        lower_vertical,
                    } => Built::Equirect(
                        xr::CompositionLayerEquirect2KHR::new()
                            .layer_flags(flags)
                            .space(space)
                            .eye_visibility(xr::EyeVisibility::BOTH)
                            .pose(layer.pose)
                            .radius(radius)
                            .central_horizontal_angle(central_horizontal)
                            .upper_vertical_angle(upper_vertical)
                            .lower_vertical_angle(lower_vertical)
                            .sub_image(sub_image()),
                    ),
                    // A cube has no sub-image: the whole swapchain is the six faces, and which way it is
                    // turned is the only thing left to say. A translation is in `placed`, which is a space at
                    // that point in the layer's own space; the orientation stays here because it does not
                    // need a space of its own.
                    wxr::LayerShape::Cube => Built::Cube(
                        xr::CompositionLayerCubeKHR::new()
                            .layer_flags(flags)
                            .space(layer.placed.as_ref().unwrap_or(space))
                            .eye_visibility(xr::EyeVisibility::BOTH)
                            .orientation(layer.pose.orientation)
                            .swapchain(&layer.swapchain)
                            .image_array_index(0),
                    ),
                })
            })
            .collect();

        // The world first and the app's pictures over it: the projection layer is what a scene is drawn into,
        // and a panel that arrived behind it would be a panel nobody sees.
        let mut present: Vec<&xr::CompositionLayerBase<'_, xr::Vulkan>> =
            Vec::with_capacity(1 + built.len());
        if let Some(layer) = &projection {
            present.push(layer);
        }
        for layer in &built {
            present.push(layer.base());
        }

        self.stream
            .end(self.predicted, self.blend, &present)
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;

        // The runtime takes its depth image back with the frame, so the next one has to take it again. This
        // is the reason [`Self::depth`] takes the image on the first ask of a frame and then reads it: this
        // line is what makes the first ask of the next frame the first ask rather than the second.
        self.measured = None;
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

        // Making a raycaster is asynchronous and a future only moves when something polls it, so this is the
        // one place it does - and this is the call a frame loop makes every frame anyway, which is why the
        // future was chosen to be finished here rather than needing a call of its own.
        if let Some(raycast) = self.raycast.as_mut() {
            raycast.poll();
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
                // An anchor the runtime has finished making, which is not a core event: `wxr::Event` has no
                // way to say "the anchor you asked for exists now", because the core's shape for that is the
                // `None` `anchor_pose` already answers until it does. This is the backend noticing, and the
                // loop carries on rather than returning - there is nothing for the caller to do about it.
                Ok(Some(xr::Event::SpatialAnchorCreateCompleteFB(event))) => {
                    if let Some(anchors) = self.anchors.as_mut() {
                        anchors.complete(event.request_id(), event.result(), event.space());
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
                    // A poll that fails is the instance going away, which is the same ending a state event
                    // would have said. Say the state first if it has not been said, then `Lost` once: a
                    // persistent error must not turn every later poll into another `Lost`, and a caller that
                    // only ever hears `Lost` never sees the state that explains it.
                    if self.state != wxr::State::Ended {
                        self.state = wxr::State::Ended;
                        return Some(wxr::Event::StateChanged(wxr::State::Ended));
                    }
                    if !self.lost {
                        self.lost = true;
                        return Some(wxr::Event::Lost);
                    }
                    return None;
                }
            }
        }
    }
}

/// One of the app's layers, before it is handed to the runtime.
///
/// They are four different structs because they are four different extensions, and the compositor is told
/// which one it is getting by the type in the header - so the shape has to survive to this point, which is
/// what [`wxr::LayerShape`] is for. `base` is the header and the chain, which is the part every layer has.
enum Built<'a> {
    Quad(xr::CompositionLayerQuad<'a, xr::Vulkan>),
    Cylinder(xr::CompositionLayerCylinderKHR<'a, xr::Vulkan>),
    Equirect(xr::CompositionLayerEquirect2KHR<'a, xr::Vulkan>),
    Cube(xr::CompositionLayerCubeKHR<'a, xr::Vulkan>),
}

impl Built<'_> {
    fn base(&self) -> &xr::CompositionLayerBase<'_, xr::Vulkan> {
        match self {
            Built::Quad(layer) => layer,
            Built::Cylinder(layer) => layer,
            Built::Equirect(layer) => layer,
            Built::Cube(layer) => layer,
        }
    }
}
