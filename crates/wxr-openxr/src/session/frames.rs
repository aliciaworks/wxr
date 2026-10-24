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
                .map_err(|error| wxr::Error::Present(format!("starting the session: {error:?}")))?;
            self.begun = true;
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

        let held = match self.swapchain.acquire_image() {
            Ok(held) => held,
            Err(error) => {
                self.abandon_frame();
                return Err(wxr::Error::Present(format!("{error:?}")));
            }
        };
        // Unbounded: the runtime is the one that knows when the image is free, and a timeout here would be
        // this code deciding it knows better.
        if let Err(error) = self.swapchain.wait_image(xr::Duration::INFINITE) {
            // The image was acquired but never handed over, so there is nothing here that may be released.
            // The frame itself still has to close, or every later `xrBeginFrame` is a call-order error.
            self.abandon_frame();
            return Err(wxr::Error::Present(format!("{error:?}")));
        }
        self.held = Some(held);

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
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        layer
            .swapchain
            .wait_image(xr::Duration::INFINITE)
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
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
        // Asking for views twice in one frame replaces them; it does not give the frame four eyes. The
        // runtime's own views and the space they were located in go with them, so a second call that fails
        // cannot leave `end` submitting the first call's eyes.
        out.views_mut().clear();
        self.located.clear();
        self.view_space = None;
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
        self.depth_asked = false;
        Ok(())
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
