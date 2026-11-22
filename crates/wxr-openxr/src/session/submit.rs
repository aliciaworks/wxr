//! Closing a frame: giving the images back, building the layers, and the one `xrEndFrame`.
//!
//! Split from `frames`, which is the frame's arrival - what the runtime is doing, where the eyes are, and
//! which images this frame holds. This is what is done with them, and the only thing it keeps is what the
//! next frame has to know.

use super::*;

impl OpenXrSession {
    pub(super) fn end_impl(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        // A frame the runtime did not want drawn was already opened and closed by `begin_impl`, because the
        // caller was told `Wait` and has nothing to hand back. Ending it again is a call-order error at the
        // runtime - an image released that was never acquired - and this is the end of a frame that was never
        // begun, which is nothing at all.
        if self.held.is_none() {
            return Ok(());
        }

        // Every image has to be given back, but a failure while doing it must not leave the frame open: a
        // frame between `xrBeginFrame` and `xrEndFrame` is one the next `xrBeginFrame` cannot start, and an
        // app that gets an error here is still going to ask for the next frame. The images that could be
        // released are; the empty end is what says this frame has nothing to draw and closes the pair.
        self.held = None;
        // Which layers this frame took an image for, taken before the release below clears `held`. A layer
        // made between `begin` and now is not one of them: nothing has been drawn into it, and its swapchain
        // has no image this frame for a sub-image to name - the runtime refuses one, and a picture nobody drew
        // is not a picture. It is acquired for the first time on the next frame, and submitted then.
        let held: Vec<bool> = self
            .layers
            .iter()
            .map(|layer| layer.as_ref().is_some_and(|layer| layer.held.is_some()))
            .collect();
        // Whether a depth image was taken this frame, read at the same moment and for the same reason: a
        // frame that took none has nothing to submit, and one whose image is already back is a chain that
        // would name an image the compositor has not been handed.
        let depth_held = self
            .target_depth
            .as_ref()
            .is_some_and(|depth| depth.held.is_some());
        // And the motion image, read before the release and only meaningful when the app asked for space
        // warp: a frame that carries an image nobody wrote would be synthesised from nothing.
        let motion_held = self
            .motion
            .as_ref()
            .is_some_and(|motion| motion.held.is_some());
        let warping = self.space_warp && motion_held && depth_held;
        let mut failed = None;
        if let Err(error) = self.swapchain.release_image() {
            failed = Some(wxr::Error::Present(format!("{error:?}").into()));
        }
        if let Err(error) = self.release_target_depth()
            && failed.is_none()
        {
            failed = Some(error);
        }
        if let Err(error) = self.release_target_motion()
            && failed.is_none()
        {
            failed = Some(error);
        }
        for layer in self.layers.iter_mut().flatten() {
            if layer.held.take().is_some()
                && let Err(error) = layer.swapchain.release_image()
                && failed.is_none()
            {
                failed = Some(wxr::Error::Present(format!("{error:?}").into()));
            }
        }
        if let Some(error) = failed {
            let _ = self.stream.end(self.predicted, self.blend, &[]);
            // The runtime has the frame either way, so this frame's depth is over too - the same clearing
            // the successful end below does.
            self.measured = None;
            self.depth_asked = false;
            return Err(error);
        }

        // The layer borrows this frame's views, so it is built here and lives only until it is submitted.
        // Each eye is a rectangle of the one image, in its own array layer - and carries the app's depth when
        // there is any to carry: see `depth_submission`.
        let extent = xr::Extent2Di {
            width: self.extent.width as i32,
            height: self.extent.height as i32,
        };
        // Both or neither, because either alone submits nothing: the image is where the values are and the
        // range is what they mean, and a compositor given one without the other would move the picture by an
        // unknown amount.
        let submitted = match (depth_held, self.depth_range) {
            (true, Some(range)) => self.target_depth.as_ref().map(|depth| (depth, range)),
            _ => None,
        };
        // Space warp is a request on top of a submission: it is chained only when the app asked and both
        // images are this frame's, so a session that never asked keeps a plain projection layer.
        let warp = match (warping, submitted, self.motion.as_ref()) {
            (true, Some((depth, range)), Some(motion)) => Some((motion, depth, range)),
            _ => None,
        };
        let (views, chain, _warp_chain) = depth_submission::projection_views(
            &self.swapchain,
            &self.located,
            extent,
            submitted,
            warp,
        );
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
            .zip(&held)
            .filter_map(|(layer, &held)| {
                if !held {
                    return None;
                }
                let layer = layer.as_ref()?;
                let space = self.spaces.get(layer.space)?;
                // The app's answer about its own alpha, kept since the layer was made: an opaque picture names
                // no bit, which is a compositor that need not sample a channel that is always one.
                let flags = super::layers::flags(layer.options);
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

        // The runtime takes its depth image back with the frame, so the next one has to take it again - which
        // is why [`super::OpenXrSession::environment_depth`] takes the image on the first ask of a frame and then reads
        // it. Cleared before the result is handed on rather than after a successful end, because a failed
        // `xrEndFrame` ends the frame just the same: `depth_asked` left true would make the next frame's first
        // ask read the buffer of the frame before instead of taking a new one.
        let ended = self
            .stream
            .end(self.predicted, self.blend, &present)
            .map_err(|error| wxr::Error::Present(format!("{error:?}").into()));
        // The depth chain is what the projection layer's views point at, so it goes after the frame does.
        drop(chain);
        self.measured = None;
        self.depth_asked = false;
        ended
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
