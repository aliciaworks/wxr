//! The session the browser handed over, and everything it is asked.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{XrFrame, XrReferenceSpace, XrSession, XrView};

use crate::convert::{
    field_of_view, hand_joint, offset_reference_space, reference_space_type, rigid, transform,
    visibility,
};
use crate::{anchors, depth, gpu, hit, input, light, planes};

impl Drop for WebXrSession {
    /// End the browser's session when this one is dropped.
    ///
    /// A WebXR session belongs to the browser, and without this it keeps it - and the display it holds - until
    /// the page goes away. Dropping the handle is how an app says it is finished, so this is where the browser
    /// is told.
    fn drop(&mut self) {
        if let Some(session) = self.inner.borrow().session.clone() {
            let _ = session.end();
        }
    }
}

/// Where the session request has got to.
pub(crate) enum Connect {
    Pending,
    Started(XrSession),
    Failed(String),
}

/// The session the browser handed over, once it arrives.
#[derive(Default)]
struct Inner {
    session: Option<XrSession>,
    /// The frame the animation callback put here, and the time it was for, until `begin` takes it.
    frame: Option<(XrFrame, Duration)>,
}

/// A reference space, which the browser hands over as a promise rather than as an object.
///
/// The promise is kept while it is pending because an offset space is made *from* another one and has to wait
/// for the same answer - which is the only reason this is not just an `Option`.
#[derive(Default)]
struct Space {
    space: Option<XrReferenceSpace>,
    promise: Option<js_sys::Promise>,
    /// The `reset` handler, kept alive for as long as the space is: a closure the browser holds and this drops
    /// is a closure that stops being called. Nothing reads it - holding it is the work.
    #[allow(dead_code)]
    on_reset: Option<Closure<dyn FnMut(web_sys::Event)>>,
}

impl Space {
    /// A space that is here.
    fn resolved(space: XrReferenceSpace) -> Self {
        Self {
            space: Some(space),
            promise: None,
            on_reset: None,
        }
    }
}

/// One of the frame's images: the colour texture, and the depth buffer the layer gave with it.
///
/// They are one thing here because the compositor hands them over as one - both come from the same sub-image -
/// and because that is what lets an importer answer for both. A layer made without a depth format has no depth
/// texture, which is why this one is an `Option`: the specification says it is nullable.
pub struct FrameImage {
    pub color: JsValue,
    pub depth: Option<JsValue>,
}

/// The binding and the layer, which only exist together: a binding with no layer presents nothing.
struct Gpu {
    binding: gpu::XrGpuBinding,
    layer: gpu::XrProjectionLayer,
}

/// A live WebXR session.
pub struct WebXrSession {
    inner: Rc<RefCell<Inner>>,
    connect: Rc<RefCell<Connect>>,
    /// The animation callback, kept alive for as long as the session is: a closure that is dropped is a
    /// closure the browser stops calling.
    callback: Option<Closure<dyn FnMut(f64, XrFrame)>>,
    /// The `end` handler's flag, and the handler itself - same reason as the callback: a closure that is
    /// dropped is a closure the browser stops calling.
    ended: Rc<Cell<bool>>,
    on_end: Option<Closure<dyn FnMut()>>,
    /// Whether `Lost` has been said, so that the end of a session is news once.
    lost: bool,
    /// Spaces, each a slot: the request is a promise, so a space exists a frame or two after it is asked for.
    spaces: Vec<Rc<RefCell<Space>>>,
    state: wxr::State,
    /// Whether the session is being shown, which is the browser's own `visibilityState`.
    visibility: wxr::Visibility,
    /// The sources this session has seen, so an id from a frame and an id from an event are the same answer.
    sources: input::Sources,
    /// The subscription to the session's six input events, once there is a session to subscribe to.
    input: Option<input::Events>,
    /// The surfaces this session has seen, for the same reason as the sources: WebXR names one by the object it
    /// is, and a core plane carries a number.
    planes: planes::Ids,
    /// The hit-test sources this session has asked for, each empty until the browser answers.
    hit_sources: Vec<Rc<RefCell<hit::Slot>>>,
    /// The light probes this session has asked for, each empty until the browser answers.
    light_probes: Vec<Rc<RefCell<light::Slot>>>,
    /// This frame's depth buffer, kept for as long as the reference into it is handed out.
    depth_image: Option<JsValue>,
    /// How much foveation the app asked for, if it asked: `None` leaves the layer's own default alone.
    foveation: Option<f32>,
    /// The anchors this session has asked for, each empty until the browser answers.
    anchors: Vec<Rc<RefCell<anchors::Slot>>>,
    /// This frame's views, kept because depth is asked for one of them by object and not by index.
    frame_views: Vec<XrView>,
    /// Reference-space `reset` events, which arrive on a space rather than on the session and are passed on
    /// from here.
    reset: Rc<RefCell<VecDeque<wxr::Event>>>,
    /// The views the frame located, for the layer the compositor would be given.
    located: usize,
    /// The device the app made, kept because a WebXR/WebGPU session needs it: the binding that hands out the
    /// textures is built from it, once, when the session arrives.
    device: Option<wgpu::Device>,
    /// The binding and the layer, when the browser gave both. `None` is a session with a head, two eyes and a
    /// clock and no picture - which is what this backend has always been able to be, and says so.
    gpu: Option<Gpu>,
    /// This frame's image, which both eyes share: one colour texture, and the depth that came with it.
    image: Option<FrameImage>,
    /// What that texture said about itself, so `images` does not have to ask it twice.
    meta: wxr::ImageMeta,
    /// The near and far planes the app asked for, kept because a session that has not arrived has no render
    /// state to put them on yet.
    depth_range: Option<(f32, f32)>,
}

impl WebXrSession {
    pub(crate) fn new(connect: Rc<RefCell<Connect>>, device: wgpu::Device) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner::default())),
            connect,
            callback: None,
            ended: Rc::new(Cell::new(false)),
            on_end: None,
            lost: false,
            spaces: Vec::new(),
            state: wxr::State::Connecting,
            visibility: wxr::Visibility::Hidden,
            sources: input::Sources::new(),
            input: None,
            planes: planes::Ids::default(),
            hit_sources: Vec::new(),
            light_probes: Vec::new(),
            depth_image: None,
            foveation: None,
            anchors: Vec::new(),
            frame_views: Vec::new(),
            reset: Rc::new(RefCell::new(VecDeque::new())),
            located: 0,
            device: Some(device),
            gpu: None,
            image: None,
            // `Unknown` rather than the default, because a session with no binding has no format and saying
            // `Rgba8Srgb` would be a plausible size dressed up as a fact - which is the one thing this
            // backend's `images` has always refused to do.
            meta: wxr::ImageMeta {
                format: wxr::ColorFormat::Unknown,
                ..Default::default()
            },
            depth_range: None,
        }
    }

    /// Ask for the binding and the layer, and keep them if the browser gives them.
    ///
    /// Three things have to be true and only one of them is this crate's to arrange: the session has to have
    /// come back with the `webgpu` feature, the device has to have been made from an XR-compatible adapter,
    /// and the browser has to have the binding at all. Any of them missing leaves a session with a head, two
    /// eyes and a clock and no picture - which is what this backend could already say for itself, so it is a
    /// warning rather than a failure.
    fn start_gpu(&mut self, session: &XrSession) {
        if self.gpu.is_some() || !gpu::has_feature(session, "webgpu") {
            return;
        }
        let Some(device) = &self.device else {
            return;
        };
        let Some(js_device) = device.as_webgpu() else {
            log::warn!(
                "wxr-webxr: the device is not a browser WebGPU one, so there are no images to give it"
            );
            return;
        };
        let binding = match gpu::XrGpuBinding::new(session, js_device.as_ref()) {
            Ok(binding) => binding,
            Err(error) => {
                log::warn!("wxr-webxr: no images: {error:?}");
                return;
            }
        };
        let init = gpu::projection_layer_init(
            &binding.get_preferred_color_format(),
            gpu::DEPTH_FORMAT_NAME,
        );
        let layer = match binding.create_projection_layer(&init) {
            Ok(layer) => layer,
            Err(error) => {
                log::warn!("wxr-webxr: the browser would not make a projection layer: {error:?}");
                return;
            }
        };
        gpu::set_layers(session, &layer);
        log::info!("wxr-webxr: a projection layer, and with it the frames");
        self.gpu = Some(Gpu { binding, layer });
        // A foveation amount asked for before there was a layer goes on now that there is one.
        self.apply_foveation();
    }

    /// Put the near and far planes on the browser's render state, if there is a session to put them on.
    ///
    /// Called again when a session arrives, which is what lets an app set them while the session is still
    /// `Connecting`: a render state belongs to the browser's session, and there is none until there is one.
    fn apply_depth_range(&self) {
        let (Some((near, far)), Some(session)) =
            (self.depth_range, self.inner.borrow().session.clone())
        else {
            return;
        };
        let state = web_sys::XrRenderStateInit::new();
        state.set_depth_near(near as f64);
        state.set_depth_far(far as f64);
        session.update_render_state_with_state(&state);
    }

    /// Put the foveation amount on the layer, when there is one - a session with no binding has no layer, and
    /// an amount it never asked for is the layer's own default and is left alone.
    fn apply_foveation(&self) {
        let (Some(gpu), Some(foveation)) = (&self.gpu, self.foveation) else {
            return;
        };
        gpu.layer.set_fixed_foveation(foveation as f64);
    }

    /// Ask for the next frame, once there is a session to ask.
    fn request_frame(&mut self) {
        let session = self.inner.borrow().session.clone();
        let Some(session) = session else {
            return;
        };
        let inner = self.inner.clone();
        let callback = Closure::wrap(Box::new(move |time: f64, frame: XrFrame| {
            // Milliseconds from the page's origin, which is the only clock a browser has.
            inner.borrow_mut().frame = Some((frame, Duration::from_secs_f64(time / 1000.0)));
        }) as Box<dyn FnMut(f64, XrFrame)>);
        // The handle comes back as a number, and a browser that will not give one has a session that is over.
        let _ = session.request_animation_frame(callback.as_ref().unchecked_ref());
        self.callback = Some(callback);
    }
}

impl wxr::Session for WebXrSession {
    /// The browser's `GPUTexture`, and the depth that came with it: a WebXR/WebGPU projection layer gives a
    /// sub-image per view that shares both and differs only in which part of the colour one the view draws
    /// into - so a frame here is one image, two viewports, and two array layers when the layer is stereo.
    type Image = FrameImage;
    /// The depth buffer, as the browser's own `GPUTexture` - which is what `gpu-optimized` depth arrives as,
    /// and only while the frame it was made for is the frame being drawn.
    type Depth = JsValue;

    fn presentation(&self) -> wxr::Presentation {
        wxr::Presentation::Composited
    }

    fn state(&self) -> wxr::State {
        self.state
    }

    fn visibility(&self) -> wxr::Visibility {
        self.visibility
    }

    fn features(&self) -> wxr::Features {
        let Some(session) = self.inner.borrow().session.clone() else {
            // Before the session arrives there is nothing to ask, and an answer that is not here yet is not a
            // capability.
            return wxr::Features::NONE;
        };
        let mut features = wxr::Features::NONE;
        for (bit, name) in [
            (wxr::Features::PLANES, "plane-detection"),
            (wxr::Features::HIT_TEST, "hit-test"),
            (wxr::Features::LIGHT_ESTIMATION, "light-estimation"),
            (wxr::Features::HAND_TRACKING, "hand-tracking"),
            (wxr::Features::ANCHORS, "anchors"),
        ] {
            if gpu::has_feature(&session, name) {
                features = features.union(bit);
            }
        }
        // Depth is granted by a feature and readable only through the binding that gives a texture: a browser
        // that granted it and no binding is a depth this backend cannot hand over.
        if gpu::has_feature(&session, "depth-sensing") && self.gpu.is_some() {
            features = features.union(wxr::Features::DEPTH);
        }
        features
    }

    fn poll(&mut self) -> Option<wxr::Event> {
        // Taken out of the slot as it is read: a session that has arrived moves into the session, and a poll
        // that left it there would find it again every time - and ask the browser for another animation frame
        // every time with it, one closure per tick, none of them ever dropped.
        let arrived = match std::mem::replace(&mut *self.connect.borrow_mut(), Connect::Pending) {
            Connect::Started(session) => Some(session),
            Connect::Failed(message) => {
                log::error!("wxr-webxr: the session was refused: {message}");
                self.state = wxr::State::Ended;
                return Some(wxr::Event::Lost);
            }
            Connect::Pending => None,
        };
        if let Some(session) = arrived {
            self.state = wxr::State::Ready;
            // Before the frame loop starts, because a WebGPU-compatible session with no layer set is a session
            // whose animation frames never arrive at all.
            self.start_gpu(&session);

            // The browser can end a session on its own - the person takes the headset off, the page loses the
            // display - and a session that does not notice is a frame loop drawing into nothing.
            let ended = self.ended.clone();
            let on_end = Closure::<dyn FnMut()>::new(move || ended.set(true));
            session.set_onend(Some(on_end.as_ref().unchecked_ref()));
            self.on_end = Some(on_end);

            // Subscribed to here because a session is the only thing that can have the events, and this is the
            // only moment there is one to ask.
            self.input = Some(input::Events::new(&session, &self.sources));

            self.inner.borrow_mut().session = Some(session);
            // A depth range the app set before the session existed goes on now that there is a render state.
            self.apply_depth_range();
            self.request_frame();
        }

        // Before the visibility, because a session that has ended has no visibility worth reading.
        if self.ended.get() {
            if self.state != wxr::State::Ended {
                self.state = wxr::State::Ended;
                return Some(wxr::Event::StateChanged(wxr::State::Ended));
            }
            if !self.lost {
                self.lost = true;
                return Some(wxr::Event::Lost);
            }
        }

        // A press the browser has already delivered is news before any tally of what is showing: it happened,
        // and the frame loop reading it a rung later would be a frame loop acting on the wrong frame.
        if let Some(events) = &self.input
            && let Some(event) = events.poll()
        {
            return Some(event);
        }

        // A space that was recentered is news the same way, from a handler that fires between frames.
        if let Some(event) = self.reset.borrow_mut().pop_front() {
            return Some(event);
        }

        // Two axes, and each is news once: the session arriving or not being here yet is the lifecycle, and
        // the browser's own `visibilityState` is the other - the same vocabulary, one rung at a time.
        let session = self.inner.borrow().session.clone();
        let next = match &session {
            None => wxr::State::Connecting,
            Some(_) => wxr::State::Ready,
        };
        if next != self.state {
            self.state = next;
            return Some(wxr::Event::StateChanged(next));
        }
        let shown = match &session {
            None => wxr::Visibility::Hidden,
            Some(session) => visibility(session.visibility_state()),
        };
        if shown != self.visibility {
            self.visibility = shown;
            return Some(wxr::Event::VisibilityChanged(shown));
        }
        None
    }

    /// What the display shows behind the picture, which WebXR calls `environmentBlendMode`.
    ///
    /// `web-sys` does not bind it - it is in the browser's own `XRSession` prototype and not in the bindings,
    /// which is how this reads it, the same way `gpu::has_feature` reads `enabledFeatures`. Opaque until there
    /// is a session to ask.
    fn blend(&self) -> wxr::Blend {
        let Some(session) = self.inner.borrow().session.clone() else {
            return wxr::Blend::Opaque;
        };
        let name = js_sys::Reflect::get(
            session.unchecked_ref::<JsValue>(),
            &JsValue::from_str("environmentBlendMode"),
        );
        match name.ok().and_then(|value| value.as_string()).as_deref() {
            Some("additive") => wxr::Blend::Additive,
            Some("alpha-blend") => wxr::Blend::AlphaBlend,
            _ => wxr::Blend::Opaque,
        }
    }

    /// The near and far planes, through the browser's own render state.
    ///
    /// Remembered as well as applied, because an app is free to set them while the session is still
    /// `Connecting` - which is the honest order when the planes are the renderer's choice and not the
    /// runtime's.
    fn set_depth_range(&mut self, near: f32, far: f32) {
        self.depth_range = Some((near, far));
        self.apply_depth_range();
    }

    fn images(&self) -> wxr::ImageMeta {
        // Read off the texture the browser handed over, so a session with no binding reports an unknown format
        // and a zero extent rather than a plausible size: a renderer that reads this and draws anyway has been
        // told the truth.
        self.meta
    }

    fn image_count(&self) -> usize {
        // One: both eyes draw into the same texture. Which part of it is a view's business, and a view says so
        // with its viewport and its layer.
        usize::from(self.image.is_some())
    }

    fn image(&self, index: usize) -> Option<&Self::Image> {
        (index == 0).then_some(self.image.as_ref()).flatten()
    }

    fn space(&mut self, kind: wxr::SpaceKind) -> Result<wxr::ReferenceSpace, wxr::Error> {
        let Some(session) = self.inner.borrow().session.clone() else {
            return Err(wxr::Error::Unavailable(
                "the session has not started yet".into(),
            ));
        };
        let promise: js_sys::Promise = session
            .request_reference_space(reference_space_type(kind))
            .unchecked_into();
        let slot = Rc::new(RefCell::new(Space {
            space: None,
            promise: Some(promise.clone()),
            on_reset: None,
        }));

        // The id the handle will have, known before the space is here so that the reset handler can name it.
        let id = self.spaces.len() as u32;
        let fill = slot.clone();
        let reset = self.reset.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(value) = JsFuture::from(promise).await
                && let Ok(space) = value.dyn_into::<XrReferenceSpace>()
            {
                // The space itself says when its origin was recentered, and every pose measured in it is stale
                // afterwards - so it is passed on as the session's news, because a core event has no space to
                // arrive on.
                let handler = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
                    reset
                        .borrow_mut()
                        .push_back(wxr::Event::Reset(wxr::ReferenceSpace::new(kind, id)));
                });
                space.set_onreset(Some(handler.as_ref().unchecked_ref()));
                let mut slot = fill.borrow_mut();
                slot.space = Some(space);
                slot.promise = None;
                slot.on_reset = Some(handler);
            }
        });

        self.spaces.push(slot);
        Ok(wxr::ReferenceSpace::new(kind, id))
    }

    fn offset_space(
        &mut self,
        space: wxr::ReferenceSpace,
        offset: wxr::Pose,
    ) -> Result<wxr::ReferenceSpace, wxr::Error> {
        let Some(base) = self.spaces.get(space.id() as usize).cloned() else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let resolved = base.borrow().space.clone();
        let pending = base.borrow().promise.clone();
        let slot = Rc::new(RefCell::new(Space::default()));
        // The base may still be a promise, and an offset of a space that is not here yet is not here yet
        // either - so the same promise is waited on, which is why a slot keeps it.
        if let Some(base) = resolved {
            *slot.borrow_mut() = Space::resolved(offset_reference_space(&base, offset)?);
        } else if let Some(promise) = pending {
            let fill = slot.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(value) = JsFuture::from(promise).await
                    && let Ok(base) = value.dyn_into::<XrReferenceSpace>()
                    && let Ok(offset) = offset_reference_space(&base, offset)
                {
                    *fill.borrow_mut() = Space::resolved(offset);
                }
            });
        } else {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        self.spaces.push(slot);
        Ok(wxr::ReferenceSpace::new(
            space.kind,
            (self.spaces.len() - 1) as u32,
        ))
    }

    fn begin(&mut self, _now: Duration, out: &mut wxr::Frame) -> Result<(), wxr::Error> {
        out.views_mut().clear();
        self.located = 0;
        // Last frame's views are not this frame's, and a depth buffer is about *that* frame's eyes.
        self.frame_views.clear();

        // The frame is whatever the last callback left behind, and the clock is the callback's own: WebXR
        // has no other.
        let Some((frame, time)) = self.inner.borrow_mut().frame.take() else {
            out.state = wxr::FrameState::Wait;
            // Asked for again, because a callback that is not re-requested is a session that stops.
            self.request_frame();
            return Ok(());
        };
        let _ = frame;
        out.predicted_display_time = time;
        out.state = wxr::FrameState::Render;
        out.views_mut().clear();
        Ok(())
    }

    fn views(
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
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
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
                Some(gpu) => {
                    let sub = gpu.binding.get_view_sub_image(&gpu.layer, &view);
                    let color = sub.color_texture();
                    let depth = sub.depth_stencil_texture();
                    let viewport = sub.viewport();
                    self.meta = gpu::image_meta(&color);
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
                        gpu::base_array_layer(&sub.get_view_descriptor()),
                    )
                }
                None => (wxr::Viewport::default(), 0),
            };
            out_views.push(wxr::View {
                eye: match view.eye() {
                    web_sys::XrEye::Left => wxr::Eye::Left,
                    web_sys::XrEye::Right => wxr::Eye::Right,
                    _ => wxr::Eye::Mono,
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

    fn inputs(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::InputSource>,
    ) -> Result<(), wxr::Error> {
        let Some(reference) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Ok(());
        };
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
            return Ok(());
        };
        let Some(session) = self.inner.borrow().session.clone() else {
            return Ok(());
        };

        // A gamepad is the `xr-standard` mapping, which is a specification of its own and is what makes
        // the button order below mean anything: the trigger first, the squeeze second, the stick's click
        // fourth, and the stick's two axes after the touchpad's. A profile that does not follow it is a
        // profile this reads the wrong way - which is a thing the gamepad's own `mapping` says, and a thing
        // to handle the day one shows up.
        let held = session.input_sources();
        for index in 0..held.length() {
            let Some(source) = held.get(index) else {
                continue;
            };
            let handedness = match source.handedness() {
                web_sys::XrHandedness::Left => wxr::Handedness::Left,
                web_sys::XrHandedness::Right => wxr::Handedness::Right,
                _ => wxr::Handedness::Unknown,
            };

            let grip = source
                .grip_space()
                .and_then(|space| frame.get_pose(&space, reference.unchecked_ref()));
            let aim = frame.get_pose(&source.target_ray_space(), reference.unchecked_ref());

            let gamepad = source.gamepad();
            let button = |index: u32| {
                gamepad.as_ref().and_then(|gamepad| {
                    gamepad
                        .buttons()
                        .get(index)
                        .dyn_into::<web_sys::GamepadButton>()
                        .ok()
                })
            };
            let axis = |index: u32| {
                gamepad
                    .as_ref()
                    .and_then(|gamepad| gamepad.axes().get(index).as_f64())
                    .unwrap_or(0.0) as f32
            };

            // A source with no grip pose is an aim and nothing to hold - a gaze cursor - and it is not
            // untracked: it has a direction, and the direction is the whole of it.
            let tracked = grip.is_some() || aim.is_some();
            out.push(wxr::InputSource {
                // The same map the event handlers use, so a frame and an event name the same source the same
                // way - which is the whole reason the core has an id where WebXR has an object.
                id: self.sources.id(&source),
                handedness,
                target_ray_mode: input::target_ray_mode(&source),
                // A source with a `hand` is one with a skeleton to ask for, which is the whole of what WebXR
                // says about it: whether the fingers are tracked is `hand`'s answer, not this one's.
                hand: source.hand().is_some(),
                grip: grip
                    .map(|pose| transform(pose.transform()))
                    .unwrap_or(wxr::Pose::IDENTITY),
                aim: aim
                    .map(|pose| transform(pose.transform()))
                    .unwrap_or(wxr::Pose::IDENTITY),
                tracked,
                buttons: wxr::Buttons {
                    select: button(0).is_some_and(|button| button.pressed()),
                    squeeze: button(1).is_some_and(|button| button.pressed()),
                    menu: button(3).is_some_and(|button| button.pressed()),
                },
                axes: wxr::Axes {
                    trigger: button(0).map(|button| button.value() as f32).unwrap_or(0.0),
                    thumbstick: wxr::glam::Vec2::new(axis(2), axis(3)),
                },
            });
        }
        Ok(())
    }

    fn hand(
        &mut self,
        source: wxr::InputId,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Hand,
    ) -> Result<(), wxr::Error> {
        out.clear();
        let Some(input) = self.sources.get(source) else {
            return Ok(());
        };
        let Some(hand) = input.hand() else {
            return Ok(());
        };
        let Some(reference) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Ok(());
        };
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
            return Ok(());
        };
        // A joint at a time, because a joint is the call the browser has - and a joint it will not answer for is
        // a joint that is not tracked, which is what `None` in an empty `Hand` already means.
        for joint in wxr::HandJoint::ALL {
            let space = hand.get(hand_joint(joint));
            if let Some(pose) = frame.get_joint_pose(&space, reference.unchecked_ref()) {
                out.joints_mut()[joint.index()] = Some(wxr::Joint {
                    pose: transform(pose.transform()),
                    radius: pose.radius(),
                });
            }
        }
        Ok(())
    }

    fn planes(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Plane>,
    ) -> Result<(), wxr::Error> {
        out.clear();
        let Some(reference) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Ok(());
        };
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
            return Ok(());
        };
        planes::detected(&frame, &reference, &self.planes, out);
        Ok(())
    }

    fn bounds(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::glam::Vec2>,
    ) -> Result<(), wxr::Error> {
        out.clear();
        let Some(space) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Ok(());
        };
        // `boundsGeometry` is on the bounded-floor space and nowhere else, so a space that is not one is not
        // this type - which is an outline with no points rather than a failure.
        let Ok(bounds) = space.dyn_into::<web_sys::XrBoundedReferenceSpace>() else {
            return Ok(());
        };
        for point in bounds.bounds_geometry().iter() {
            let Ok(point) = point.dyn_into::<web_sys::DomPointReadOnly>() else {
                continue;
            };
            out.push(wxr::glam::Vec2::new(point.x() as f32, point.z() as f32));
        }
        Ok(())
    }

    fn hit_test_source(
        &mut self,
        space: wxr::ReferenceSpace,
    ) -> Result<wxr::HitTestSource, wxr::Error> {
        let Some(session) = self.inner.borrow().session.clone() else {
            return Err(wxr::Error::Unavailable(
                "the session has not started yet".into(),
            ));
        };
        let Some(base) = self.spaces.get(space.id() as usize).cloned() else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let slot = Rc::new(RefCell::new(hit::Slot::default()));
        // The space may still be a promise, and a ray out of a space that is not here is not here either - so
        // the request waits on the same promise the space does.
        let resolved = base.borrow().space.clone();
        if let Some(base) = resolved {
            hit::request(&session, &base, slot.clone());
        } else if let Some(promise) = base.borrow().promise.clone() {
            let fill = slot.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(value) = JsFuture::from(promise).await
                    && let Ok(base) = value.dyn_into::<XrReferenceSpace>()
                {
                    hit::request(&session, &base, fill);
                }
            });
        } else {
            return Err(wxr::Error::NoSpace(space.kind));
        }
        self.hit_sources.push(slot);
        Ok(wxr::HitTestSource::new((self.hit_sources.len() - 1) as u32))
    }

    fn hits(
        &mut self,
        source: wxr::HitTestSource,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::Hit>,
    ) -> Result<(), wxr::Error> {
        out.clear();
        let Some(slot) = self.hit_sources.get(source.id() as usize) else {
            return Ok(());
        };
        let Some(source) = slot.borrow().source() else {
            return Ok(());
        };
        let Some(base) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Ok(());
        };
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
            return Ok(());
        };
        hit::results(&frame, &source, &base, out);
        Ok(())
    }

    fn light_probe(&mut self) -> Result<wxr::LightProbe, wxr::Error> {
        let Some(session) = self.inner.borrow().session.clone() else {
            return Err(wxr::Error::Unavailable(
                "the session has not started yet".into(),
            ));
        };
        let slot = Rc::new(RefCell::new(light::Slot::default()));
        light::request(&session, slot.clone());
        self.light_probes.push(slot);
        Ok(wxr::LightProbe::new((self.light_probes.len() - 1) as u32))
    }

    fn light(
        &mut self,
        probe: wxr::LightProbe,
        out: &mut wxr::LightEstimate,
    ) -> Result<(), wxr::Error> {
        *out = wxr::LightEstimate::default();
        let Some(slot) = self.light_probes.get(probe.id() as usize) else {
            return Ok(());
        };
        let Some(probe) = slot.borrow().probe() else {
            return Ok(());
        };
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
            return Ok(());
        };
        light::estimate(&frame, &probe, out);
        Ok(())
    }

    fn set_foveation(&mut self, amount: f32) {
        // Clamped, because the specification says a value outside `0..=1` is clamped rather than refused.
        self.foveation = Some(amount.clamp(0.0, 1.0));
        self.apply_foveation();
    }

    fn request_viewport_scale(&mut self, view: usize, scale: Option<f32>) {
        // `None` is ignored, which the specification says so that a recommendation may be passed unchecked.
        // It lands when the browser next answers for this view's viewport, which is the next `views`.
        if let Some(view) = self.frame_views.get(view) {
            view.request_viewport_scale(scale.map(|scale| scale as f64));
        }
    }

    fn anchor(
        &mut self,
        space: wxr::ReferenceSpace,
        pose: wxr::Pose,
    ) -> Result<wxr::Anchor, wxr::Error> {
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
            return Err(wxr::Error::Unavailable("a frame has not begun yet".into()));
        };
        let Some(base) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let slot = Rc::new(RefCell::new(anchors::Slot::default()));
        anchors::create(&frame, &base, rigid(pose)?, slot.clone());
        self.anchors.push(slot);
        Ok(wxr::Anchor::new((self.anchors.len() - 1) as u32))
    }

    fn anchor_pose(
        &mut self,
        anchor: wxr::Anchor,
        space: wxr::ReferenceSpace,
    ) -> Result<Option<wxr::Pose>, wxr::Error> {
        let Some(anchor) = self
            .anchors
            .get(anchor.id() as usize)
            .and_then(|slot| slot.borrow().anchor())
        else {
            return Ok(None);
        };
        let Some(base) = self
            .spaces
            .get(space.id() as usize)
            .and_then(|slot| slot.borrow().space.clone())
        else {
            return Ok(None);
        };
        let Some((frame, _)) = self.inner.borrow().frame.clone() else {
            return Ok(None);
        };
        Ok(anchors::pose(&frame, &anchor, &base))
    }

    fn release_anchor(&mut self, anchor: wxr::Anchor) {
        let Some(slot) = self.anchors.get(anchor.id() as usize) else {
            return;
        };
        if let Some(anchor) = slot.borrow().anchor() {
            anchor.delete();
        }
        slot.borrow_mut().forget();
    }

    fn depth(&mut self, view: usize) -> Option<(&Self::Depth, wxr::DepthInfo)> {
        self.depth_image = None;
        // From the binding and not the frame, because this session asked for GPU depth - the one that arrives
        // as something to draw with rather than as bytes to read.
        let gpu = self.gpu.as_ref()?;
        let view = self.frame_views.get(view)?;
        let (image, info) = depth::information(&gpu.binding, view)?;
        self.depth_image = Some(image);
        self.depth_image.as_ref().map(|image| (image, info))
    }

    fn end(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        // There is nothing to hand back: the compositor has no image of ours. The next frame is asked for,
        // which is what keeps the session running.
        self.request_frame();
        Ok(())
    }
}
