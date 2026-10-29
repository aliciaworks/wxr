//! One frame of a browser session: begun, drawn into, and given back.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here.

use super::*;

impl WebXrSession {
    /// The frame this session is drawing into, which is the one `begin` took out of the slot.
    ///
    /// Every one of these readers used to look in the slot the animation callback fills instead, and every one
    /// of them got nothing back: `begin` *takes* that slot, so between `begin` and `end` it is empty. `views`
    /// was fixed - a session that draws no views is a session that visibly does nothing - and the rest had
    /// been answering "no inputs, no joints, no planes, no hits, no light, no anchors" ever since, each of
    /// them a silent `Ok` that looked like a session with none of those things rather than one that could not
    /// find its own frame. This is the one place that asks, so that the next one cannot ask the wrong place.
    pub(super) fn frame(&self) -> Option<XrFrame> {
        self.current.clone()
    }

    /// Ask for the next frame, once there is a session to ask.
    ///
    /// Here rather than beside the other session setup because this is the half of the frame loop the
    /// browser owns: one request per callback, and the callback is what puts the frame in the slot `begin`
    /// takes. It is kept alive in `self.callback` for as long as the browser may call it.
    pub(super) fn request_frame(&mut self) {
        // A session that has ended has no next animation frame, and asking is the one throw the browser
        // gives this call: `InvalidStateError`. The end handler may have fired between the last callback and
        // this one, so the state alone is not enough.
        if self.state == wxr::State::Ended || self.ended.get() {
            return;
        }
        let session = self.inner.borrow().session.clone();
        let Some(session) = session else {
            return;
        };
        let inner = self.inner.clone();
        {
            let mut borrow = inner.borrow_mut();
            if borrow.awaiting {
                return;
            }
            borrow.awaiting = true;
        }
        let callback_inner = inner.clone();
        let callback = Closure::wrap(Box::new(move |time: f64, frame: XrFrame| {
            // Milliseconds from the page's origin, which is the only clock a browser has. This one has arrived,
            // so the next one can be asked for.
            let mut inner = callback_inner.borrow_mut();
            inner.awaiting = false;
            inner.frame = Some((frame, Duration::from_secs_f64(time / 1000.0)));
        }) as Box<dyn FnMut(f64, XrFrame)>);
        // Caught, because the session can end between two frames: without this the throw lands in whatever
        // called `end` and the frame loop stops by exception rather than by the `Ended` every other path
        // reports. A refused request is also not one that will ever call back, so `awaiting` is cleared and
        // the end handler's flag is set - the next `poll` then says `Ended` and `Lost` in that order.
        match throws::request_animation_frame(&session, callback.as_ref().unchecked_ref()) {
            Ok(_) => self.callback = Some(callback),
            Err(error) => {
                log::debug!("wxr-webxr: the browser would not give another frame: {error:?}");
                inner.borrow_mut().awaiting = false;
                self.ended.set(true);
            }
        }
    }
}

impl WebXrSession {
    /// Forget the frame this session was holding, and everything that belonged to it.
    ///
    /// Called when a frame is begun and when it is given back, because those are the two ends of the only
    /// window in which a browser's `XRFrame` and its images are valid. A frame left in the slot between them
    /// is a frame `views`, `depth` or `anchor` would answer from after the browser had taken it back.
    fn clear_frame(&mut self) {
        self.current = None;
        self.located = 0;
        self.frame_views.clear();
        self.layer_images.clear();
        self.image = None;
        self.meta = wxr::ImageMeta {
            format: wxr::ColorFormat::Unknown,
            ..Default::default()
        };
        self.depth_image = None;
    }

    pub(super) fn begin_impl(
        &mut self,
        _now: Duration,
        out: &mut wxr::Frame,
    ) -> Result<(), wxr::Error> {
        out.reset();
        self.clear_frame();

        // A session that has ended does not get a frame, even if the browser left one queued before it said
        // so: the queued callback is the last one there will be, and drawing it would be a picture after the
        // session it belonged to was gone. `Exit` is what the core's frame loop is allowed to stop on.
        if self.state == wxr::State::Ended {
            out.state = wxr::FrameState::Exit;
            return Ok(());
        }

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
        Ok(())
    }
}

impl WebXrSession {
    pub(super) fn end_impl(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        // There is nothing to hand back to the browser: the frame ends when this call returns, so its views,
        // its picture and its depth all stop being answers here rather than at the next `begin`. The next
        // frame is asked for, which is what keeps the session running.
        self.clear_frame();
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
        // This frame's eyes replace the last answer rather than adding to it. The same is true of the
        // browser's views the depth pass is indexed by: a retry that returned early must not leave a count
        // that names views no longer in the frame.
        out.views_mut().clear();
        self.frame_views.clear();
        self.located = 0;
        let Some(reference) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            // The space is still a promise, which is not an error: it is one or two frames of a session
            // that just started.
            return Ok(());
        };
        let Some(frame) = self.frame() else {
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
                    // A sub-image the browser will not give is a view with no place to draw, not a view with
                    // a default place: a zero viewport is a pass wgpu refuses, and the other eye's image is
                    // not this eye's. Leave it out, the way a pose the frame will not give is left out above,
                    // and keep the indices of the views that did answer together.
                    Err(error) => {
                        log::warn!("wxr-webxr: no sub-image for a view: {error:?}");
                        continue;
                    }
                },
                // No binding is a session with a head and no picture, which is still a real view list - an
                // app drawing into its own canvas uses it.
                None => (wxr::Viewport::default(), 0),
            };
            // Kept as the browser's own object, because `getDepthInformation` is asked one of these and not an
            // index into anything this core has. It goes in only when the view does, so the index `depth`
            // takes and the index the renderer has are the same.
            self.frame_views.push(view.clone());
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
