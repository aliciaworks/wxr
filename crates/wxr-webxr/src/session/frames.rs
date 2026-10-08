//! One frame of a browser session: begun, drawn into, and given back.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here.

use super::*;

impl WebXrSession {
    pub(super) fn begin_impl(
        &mut self,
        _now: Duration,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        out.views_mut().clear();
        self.located = 0;
        // Last frame's views are not this frame's, and a depth buffer is about *that* frame's eyes. The same
        // is true of a layer's picture: it is the compositor's for one frame and taken back at the end of it.
        self.frame_views.clear();
        self.layer_images.clear();

        // The frame is whatever the last callback left behind, and the clock is the callback's own: WebXR
        // has no other.
        let Some((frame, time)) = self.inner.borrow_mut().frame.take() else {
            out.state = wxr::FrameState::Wait;
            // Asked for again, because a callback that is not re-requested is a session that stops.
            self.request_frame();
            return Ok(());
        };
        self.current = Some(frame);
        out.predicted_display_time = time;
        out.state = wxr::FrameState::Render;
        out.views_mut().clear();
        Ok(())
    }
}

impl WebXrSession {
    pub(super) fn end_impl(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        // There is nothing to hand back: the compositor has no image of ours. The next frame is asked for,
        // which is what keeps the session running.
        self.request_frame();
        Ok(())
    }
}

impl WebXrSession {
    pub(super) fn views_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        let Some(reference) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            // The space is still a promise, which is not an error: it is one or two frames of a session
            // that just started.
            return Ok(());
        };
        let Some(frame) = self.current.clone() else {
            return Ok(());
        };
        let Some(pose) = frame.get_viewer_pose(&reference) else {
            return Ok(());
        };

        // The head, which WebXR reports beside the eyes: `transform` is `XRViewerPose.transform`, and the views
        // below are placed around it.
        out.viewer = transform(pose.transform());

        let views = pose.views();
        let out_views = out.views_mut();
        for index in 0..views.length() {
            let Ok(view) = views.get(index).dyn_into::<XrView>() else {
                continue;
            };
            // Kept as the browser's own object, because `getDepthInformation` is asked one of these and not an
            // index into anything this core has.
            self.frame_views.push(view.clone());
            // What this view draws into. With a layer, a sub-image per view says which part of the one
            // texture and which array layer - and the texture itself is taken here too, so that `images` has
            // it by the time the renderer asks.
            let (viewport, layer) = match &self.gpu {
                Some(gpu) => match throws::get_view_sub_image(&gpu.binding, &gpu.layer, &view) {
                    Ok(sub) => {
                        let color = sub.color_texture();
                        let depth = sub.depth_stencil_texture();
                        let viewport = sub.viewport();
                        self.meta = image_meta(&color);
                        self.image = Some(FrameImage {
                            color,
                            depth: (!depth.is_null_or_undefined()).then_some(depth),
                        });
                        (
                            wxr::Viewport {
                                x: viewport.x().max(0) as u32,
                                y: viewport.y().max(0) as u32,
                                width: viewport.width().max(0) as u32,
                                height: viewport.height().max(0) as u32,
                            },
                            base_array_layer(&sub.get_view_descriptor()),
                        )
                    }
                    // A sub-image the browser will not give is a frame with no picture, which is the same
                    // answer as a session that never had a binding.
                    Err(error) => {
                        log::warn!("wxr-webxr: no sub-image for a view: {error:?}");
                        (wxr::Viewport::default(), 0)
                    }
                },
                None => (wxr::Viewport::default(), 0),
            };
            out_views.push(wxr::View {
                eye: match view.eye() {
                    XrEye::Left => wxr::Eye::Left,
                    XrEye::Right => wxr::Eye::Right,
                    XrEye::None | XrEye::__Invalid => wxr::Eye::Mono,
                },
                pose: transform(view.transform()),
                fov: field_of_view(&view.projection_matrix()),
                viewport,
                image: 0,
                layer,
                recommended_viewport_scale: view
                    .recommended_viewport_scale()
                    .map(|scale| scale as f32),
            });
        }
        self.located = out_views.len();
        Ok(())
    }
}
