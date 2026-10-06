//! A WebXR backend for wxr, for the browser.
//!
//! WebXR is the one of the three platforms that is a *specification* rather than a vendor's API, which is
//! why the core is shaped like it - and it is also the one that does not fit a synchronous session. Three
//! things arrive as promises or as callbacks instead of as calls:
//!
//! * `requestSession` is a promise, so a session cannot be connected to in one call. The core has
//!   `wxr::State::Connecting` for exactly this: the request is started, and the caller polls until it lands
//!   or fails.
//! * `requestReferenceSpace` is a promise too, so a space is *asked for* before it exists. A space here is
//!   a slot: a caller that asks for one and draws in the same breath gets no views for a frame or two,
//!   which is what an empty view list means.
//! * A frame arrives in the animation callback rather than from a `wait`. The callback puts it in a slot,
//!   `wxr::Session::begin` drains it, and the next one is asked for when the frame is handed back.
//!
//! And there is one thing this cannot do at all yet: **images**. WebXR's binding for WebGPU
//! (`XRGPUBinding`) is not in `web-sys`, and in a browser it is behind the `webxr-webgpu-binding` flag -
//! a developer feature that has to be asked for, which *does* work on Linux even though the announcement
//! named Windows and Android. The WebGL one gives a framebuffer that wgpu cannot draw into. So the session
//! is `wxr::Presentation::Composited` with no images: it hands a renderer the head, the eyes and the timing,
//! and where the picture goes until the binding is on by default is the renderer's own canvas. Saying that
//! plainly is better than an `Image` type that is not one.
//!
//! Two things about that binding are worth writing down before it is wired in, because both are surprises.
//! A WebGPU-compatible session is **layers-only**: `baseLayer` must not be set, and a projection layer made
//! through `XRGPUBinding` is what a session needs - *without* a layer, `requestAnimationFrame` calls back
//! zero times, which is the spec's design and not a bug in anybody's code. And such a session reports
//! projection matrices in a `0..w` clip depth range instead of WebGL's `-w..w`, so the conversion a WebGL
//! session needs is the wrong one for it.
//!
//! The explainer's three steps, for whoever wires this up, are: `new XRGPUBinding(session, device)`,
//! `binding.createProjectionLayer({ colorFormat: binding.getPreferredColorFormat() })`, and
//! `session.updateRenderState({ layers: [layer] })` - after which a frame's `binding.getViewSubImage(layer,
//! view)` answers with the **same** colour and depth textures for both eyes and a *per-view* texture view
//! descriptor and viewport. That last part is the same shape the core already carries: `View::viewport` is
//! exactly what a sub-image reports, and the two eyes are views of one texture rather than two textures.
//!
//! **The import path for those images exists, and so does the history of why it does not work yet.** wgpu's
//! ["Add WebGPU backend interop for WebXR integration"](https://github.com/gfx-rs/wgpu/pull/9350) set out to
//! add three things and landed two: `Device::as_webgpu` and `Device::create_texture_from_webgpu_handle` - both
//! in the wgpu this workspace is on - while `RequestAdapterOptions::xr_compatible` was dropped for being a
//! breaking change to a public struct. Without that field a wgpu device cannot be an XR-compatible one, so it
//! cannot be given to `XRGPUBinding`, and the images stay the browser's. It is a known and agreed gap rather
//! than a mystery: wgpu's [issue #8329](https://github.com/gfx-rs/wgpu/issues/8329) is where the shape of the
//! fix was settled - forward the flag on the web, ignore it on native. `WebXr::gpu_binding` is what an app
//! can ask in the meantime, and the three things a session will need are written down above.

#![cfg(target_family = "wasm")]

mod gpu;
mod import;
mod input;

pub use import::Images;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    XrFrame, XrHandJoint, XrReferenceSpace, XrReferenceSpaceType, XrRigidTransform, XrSession,
    XrSessionMode,
};

use wxr::glam::{Quat, Vec3};

/// What the renderer made.
///
/// Nothing is handed to the runtime yet, because there is no binding to hand it to - but the type exists,
/// because a renderer written against one backend is written against all of them, and because the day the
/// WebGPU binding ships this is where the sub-images will be made from.
pub struct Device {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
}

/// The browser's XR system, if the browser has one.
pub struct WebXr {
    system: web_sys::XrSystem,
}

impl WebXr {
    /// The `navigator.xr` of the page. A browser without WebXR is
    /// [`wxr::Error::Unavailable`] rather than a panic, and this is deliberately not "is a headset
    /// connected": that is a question for [`WebXr::is_supported`].
    pub fn load() -> Result<Self, Error> {
        let navigator = web_sys::window().ok_or(Error::NoWindow)?.navigator();
        // web-sys hands back the type whether or not the browser has one, so the question is asked of the
        // value: a browser without WebXR gives an `undefined` where the object should be.
        let system = navigator.xr();
        if JsValue::from(system.clone()).is_undefined() {
            return Err(Error::NoXr);
        }
        Ok(Self { system })
    }

    /// Whether the browser can start a session of this kind, which is an async question with no synchronous
    /// answer - the promise is the answer.
    pub fn is_supported(&self, mode: XrSessionMode) -> js_sys::Promise<js_sys::Boolean> {
        self.system.is_session_supported(mode)
    }

    /// Whether the page has the WebXR/WebGPU binding at all.
    ///
    /// `XRGPUBinding` is in Chromium behind the `webxr-webgpu-binding` flag, so this is a question with two
    /// answers on the same browser run twice. It says the *browser* could hand over images; it does not say
    /// this crate can use them, and the module documentation says exactly where that stands - one missing
    /// field in wgpu's adapter options, and not on this side of the boundary.
    pub fn gpu_binding() -> bool {
        js_sys::Reflect::has(&js_sys::global(), &JsValue::from_str("XRGPUBinding")).unwrap_or(false)
    }
}

impl wxr::Backend for WebXr {
    type Device = Device;
    type Session = WebXrSession;

    fn connect(&self, device: Device, mode: wxr::SessionMode) -> Result<WebXrSession, wxr::Error> {
        // The session is asked for and *not* waited for: blocking on a promise in a browser is not a thing
        // that can be done, and a `Connecting` session that is polled is the same ladder a session is
        // climbed by anyway.
        let result = Rc::new(RefCell::new(Connect::Pending));
        // The options form, because `local-floor` is asked for and not optional: a runtime that cannot say
        // where the floor is would put the player's feet at their eyes, and a session that is refused for
        // asking is a session that was never going to be usable.
        let init = web_sys::XrSessionInit::new();
        // A floor is asked for only where there is a room to have one: an inline session has no floor, and a
        // required feature a runtime will not grant is a session that does not exist at all.
        if mode != wxr::SessionMode::Inline {
            init.set_required_features(&[JsValue::from_str("local-floor")]);
        }
        // `webgpu` is asked for as an *optional* feature: a browser that will not grant it is a browser that
        // renders WebGL, and a required feature that is not there is a session that does not exist at all. What
        // came back is what `gpu::has_feature` asks about before any of the binding is attempted.
        init.set_optional_features(&[JsValue::from_str("webgpu")]);
        let requested = self
            .system
            .request_session_with_options(session_mode(mode), &init);
        let requested: js_sys::Promise = requested.unchecked_into();

        let slot = result.clone();
        wasm_bindgen_futures::spawn_local(async move {
            *slot.borrow_mut() = match JsFuture::from(requested).await {
                Ok(value) => match value.dyn_into::<XrSession>() {
                    Ok(session) => Connect::Started(session),
                    Err(value) => Connect::Failed(format!("the runtime returned {:?}", value)),
                },
                Err(error) => Connect::Failed(format!("{error:?}")),
            };
        });

        Ok(WebXrSession::new(result, device.device))
    }
}

/// What went wrong before a session existed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("there is no window, so there is nothing to ask for a session")]
    NoWindow,
    #[error("this browser has no WebXR, and there is no other way to a headset from a page")]
    NoXr,
}

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

/// A browser's visibility state in the core's terms, which is a translation and not a mapping.
///
/// It is the same vocabulary: WebXR is where the core's [`wxr::Visibility`] came from, three rungs and all.
/// Nothing is folded and nothing is invented.
fn visibility(state: web_sys::XrVisibilityState) -> wxr::Visibility {
    match state {
        web_sys::XrVisibilityState::Visible => wxr::Visibility::Visible,
        web_sys::XrVisibilityState::VisibleBlurred => wxr::Visibility::VisibleBlurred,
        _ => wxr::Visibility::Hidden,
    }
}

impl From<Error> for wxr::Error {
    fn from(error: Error) -> Self {
        wxr::Error::Unavailable(error.to_string())
    }
}

/// Where the session request has got to.
enum Connect {
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
}

impl Space {
    /// A space that is here.
    fn resolved(space: XrReferenceSpace) -> Self {
        Self {
            space: Some(space),
            promise: None,
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
    fn new(connect: Rc<RefCell<Connect>>, device: wgpu::Device) -> Self {
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

    fn presentation(&self) -> wxr::Presentation {
        wxr::Presentation::Composited
    }

    fn state(&self) -> wxr::State {
        self.state
    }

    fn visibility(&self) -> wxr::Visibility {
        self.visibility
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
        }));

        let fill = slot.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(value) = JsFuture::from(promise).await
                && let Ok(space) = value.dyn_into::<XrReferenceSpace>()
            {
                *fill.borrow_mut() = Space::resolved(space);
            }
        });

        self.spaces.push(slot);
        Ok(wxr::ReferenceSpace::new(
            kind,
            (self.spaces.len() - 1) as u32,
        ))
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
            let Ok(view) = views.get(index).dyn_into::<web_sys::XrView>() else {
                continue;
            };
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

    fn end(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        // There is nothing to hand back: the compositor has no image of ours. The next frame is asked for,
        // which is what keeps the session running.
        self.request_frame();
        Ok(())
    }
}

/// Which browser session mode a core one is, which is WebXR's own three values.
fn session_mode(mode: wxr::SessionMode) -> XrSessionMode {
    match mode {
        wxr::SessionMode::Inline => XrSessionMode::Inline,
        wxr::SessionMode::ImmersiveVr => XrSessionMode::ImmersiveVr,
        wxr::SessionMode::ImmersiveAr => XrSessionMode::ImmersiveAr,
    }
}

/// Which WebXR reference space a core one is.
fn reference_space_type(kind: wxr::SpaceKind) -> XrReferenceSpaceType {
    match kind {
        wxr::SpaceKind::Viewer => XrReferenceSpaceType::Viewer,
        wxr::SpaceKind::Local => XrReferenceSpaceType::Local,
        wxr::SpaceKind::LocalFloor => XrReferenceSpaceType::LocalFloor,
        wxr::SpaceKind::BoundedFloor => XrReferenceSpaceType::BoundedFloor,
        // WebXR's unbounded space is a first-class one, which is the other way round from OpenXR.
        wxr::SpaceKind::Unbounded => XrReferenceSpaceType::Unbounded,
    }
}

/// A space at `offset` inside `base`, which is WebXR's `getOffsetReferenceSpace`.
fn offset_reference_space(
    base: &XrReferenceSpace,
    offset: wxr::Pose,
) -> Result<XrReferenceSpace, wxr::Error> {
    let transform = rigid(offset)?;
    Ok(base.get_offset_reference_space(&transform))
}

/// The core's pose as the browser's rigid transform.
///
/// A position is a point and a quaternion one with four coordinates, which is the shape the constructor asks
/// for - the constructor, because an `XRRigidTransform` has no setters worth using.
fn rigid(pose: wxr::Pose) -> Result<XrRigidTransform, wxr::Error> {
    let position = web_sys::DomPointInit::new();
    position.set_x(pose.position.x as f64);
    position.set_y(pose.position.y as f64);
    position.set_z(pose.position.z as f64);
    let orientation = web_sys::DomPointInit::new();
    orientation.set_x(pose.orientation.x as f64);
    orientation.set_y(pose.orientation.y as f64);
    orientation.set_z(pose.orientation.z as f64);
    orientation.set_w(pose.orientation.w as f64);
    XrRigidTransform::new_with_position_and_orientation(&position, &orientation)
        .map_err(|error| wxr::Error::Rejected(format!("{error:?}")))
}

/// Which browser joint a core one is.
///
/// A one-to-one match, because it is WebXR's own list: the core took the names from the specification rather
/// than inventing a vocabulary of its own.
fn hand_joint(joint: wxr::HandJoint) -> XrHandJoint {
    match joint {
        wxr::HandJoint::Wrist => XrHandJoint::Wrist,
        wxr::HandJoint::ThumbMetacarpal => XrHandJoint::ThumbMetacarpal,
        wxr::HandJoint::ThumbPhalanxProximal => XrHandJoint::ThumbPhalanxProximal,
        wxr::HandJoint::ThumbPhalanxDistal => XrHandJoint::ThumbPhalanxDistal,
        wxr::HandJoint::ThumbTip => XrHandJoint::ThumbTip,
        wxr::HandJoint::IndexFingerMetacarpal => XrHandJoint::IndexFingerMetacarpal,
        wxr::HandJoint::IndexFingerPhalanxProximal => XrHandJoint::IndexFingerPhalanxProximal,
        wxr::HandJoint::IndexFingerPhalanxIntermediate => {
            XrHandJoint::IndexFingerPhalanxIntermediate
        }
        wxr::HandJoint::IndexFingerPhalanxDistal => XrHandJoint::IndexFingerPhalanxDistal,
        wxr::HandJoint::IndexFingerTip => XrHandJoint::IndexFingerTip,
        wxr::HandJoint::MiddleFingerMetacarpal => XrHandJoint::MiddleFingerMetacarpal,
        wxr::HandJoint::MiddleFingerPhalanxProximal => XrHandJoint::MiddleFingerPhalanxProximal,
        wxr::HandJoint::MiddleFingerPhalanxIntermediate => {
            XrHandJoint::MiddleFingerPhalanxIntermediate
        }
        wxr::HandJoint::MiddleFingerPhalanxDistal => XrHandJoint::MiddleFingerPhalanxDistal,
        wxr::HandJoint::MiddleFingerTip => XrHandJoint::MiddleFingerTip,
        wxr::HandJoint::RingFingerMetacarpal => XrHandJoint::RingFingerMetacarpal,
        wxr::HandJoint::RingFingerPhalanxProximal => XrHandJoint::RingFingerPhalanxProximal,
        wxr::HandJoint::RingFingerPhalanxIntermediate => XrHandJoint::RingFingerPhalanxIntermediate,
        wxr::HandJoint::RingFingerPhalanxDistal => XrHandJoint::RingFingerPhalanxDistal,
        wxr::HandJoint::RingFingerTip => XrHandJoint::RingFingerTip,
        wxr::HandJoint::PinkyFingerMetacarpal => XrHandJoint::PinkyFingerMetacarpal,
        wxr::HandJoint::PinkyFingerPhalanxProximal => XrHandJoint::PinkyFingerPhalanxProximal,
        wxr::HandJoint::PinkyFingerPhalanxIntermediate => {
            XrHandJoint::PinkyFingerPhalanxIntermediate
        }
        wxr::HandJoint::PinkyFingerPhalanxDistal => XrHandJoint::PinkyFingerPhalanxDistal,
        wxr::HandJoint::PinkyFingerTip => XrHandJoint::PinkyFingerTip,
    }
}

/// A transform in the core's terms.
fn transform(transform: web_sys::XrRigidTransform) -> wxr::Pose {
    let position = transform.position();
    let orientation = transform.orientation();
    // The browser reports these as doubles, and a pose is `f32` everywhere else.
    wxr::Pose {
        position: Vec3::new(
            position.x() as f32,
            position.y() as f32,
            position.z() as f32,
        ),
        orientation: Quat::from_xyzw(
            orientation.x() as f32,
            orientation.y() as f32,
            orientation.z() as f32,
            orientation.w() as f32,
        ),
    }
}

/// A field of view, derived from the projection matrix.
///
/// WebXR does not report angles: it reports the matrix the eyes are projected with, and the four openings
/// have to be read back out of it. The matrix is column-major and this is the standard inverse - the
/// horizontal scale is the first element, the vertical the sixth, and the two offsets are what make the
/// projection asymmetric.
fn field_of_view(projection: &[f32]) -> wxr::FieldOfView {
    let at = |index: usize| projection[index];
    let (x_scale, y_scale) = (at(0), at(5));
    let (x_offset, y_offset) = (at(8), at(9));
    if x_scale == 0.0 || y_scale == 0.0 {
        return wxr::FieldOfView::symmetric(0.0, 0.0);
    }
    wxr::FieldOfView {
        up: ((y_offset + 1.0) / y_scale).atan(),
        down: (-((y_offset - 1.0) / y_scale)).atan(),
        left: (-((x_offset - 1.0) / x_scale)).atan(),
        right: ((x_offset + 1.0) / x_scale).atan(),
    }
}
