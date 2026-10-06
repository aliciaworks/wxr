//! A WebXR backend for wxr, for the browser.
//!
//! WebXR is the one of the three platforms that is a *specification* rather than a vendor's API, which is
//! why the core is shaped like it - and it is also the one that does not fit a synchronous session. Three
//! things arrive as promises or as callbacks instead of as calls:
//!
//! * `requestSession` is a promise, so a session cannot be connected to in one call. The core has
//!   `wxr::State::Connecting` for exactly this: the request is started, and the caller polls the ladder
//!   until it lands or fails.
//! * `requestReferenceSpace` is a promise too, so a space is *asked for* before it exists. A space here is
//!   a slot: a caller that asks for one and draws in the same breath gets no views for a frame or two,
//!   which is what an empty view list means.
//! * A frame arrives in the animation callback rather than from a `wait`. The callback puts it in a slot,
//!   `wxr::Session::begin` drains it, and the next one is asked for when the frame is handed back.
//!
//! And there is one thing this cannot do at all yet: **images**. WebXR's binding for WebGPU
//! (`XRGPUBinding`) is not in `web-sys` and not in browsers, and the WebGL one gives a framebuffer that
//! wgpu cannot draw into. So the session is `wxr::Presentation::Composited` with no images: it hands a
//! renderer the head, the eyes and the timing, and where the picture goes until the binding ships is the
//! renderer's own canvas. Saying that plainly is better than an `Image` type that is not one.

#![cfg(target_family = "wasm")]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{XrFrame, XrReferenceSpace, XrReferenceSpaceType, XrSession, XrSessionMode};

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
}

impl wxr::Backend for WebXr {
    type Device = Device;
    type Session = WebXrSession;

    fn connect(&self, _device: Device) -> Result<WebXrSession, wxr::Error> {
        // The session is asked for and *not* waited for: blocking on a promise in a browser is not a thing
        // that can be done, and a `Connecting` session that is polled is the same ladder a session is
        // climbed by anyway.
        let result = Rc::new(RefCell::new(Connect::Pending));
        // The options form, because `local-floor` is asked for and not optional: a runtime that cannot say
        // where the floor is would put the player's feet at their eyes, and a session that is refused for
        // asking is a session that was never going to be usable.
        let init = web_sys::XrSessionInit::new();
        init.set_required_features(&[JsValue::from_str("local-floor")]);
        let requested = self
            .system
            .request_session_with_options(XrSessionMode::ImmersiveVr, &init);
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

        Ok(WebXrSession::new(result))
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

/// A live WebXR session.
pub struct WebXrSession {
    inner: Rc<RefCell<Inner>>,
    connect: Rc<RefCell<Connect>>,
    /// The animation callback, kept alive for as long as the session is: a closure that is dropped is a
    /// closure the browser stops calling.
    callback: Option<Closure<dyn FnMut(f64, XrFrame)>>,
    /// Spaces, each a slot: the request is a promise, so a space exists a frame or two after it is asked for.
    spaces: Vec<Rc<RefCell<Option<XrReferenceSpace>>>>,
    state: wxr::State,
    /// The views the frame located, for the layer the compositor would be given.
    located: usize,
}

impl WebXrSession {
    fn new(connect: Rc<RefCell<Connect>>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner::default())),
            connect,
            callback: None,
            spaces: Vec::new(),
            state: wxr::State::Connecting,
            located: 0,
        }
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
    /// A number the browser's sub-image would have. There is no image yet - see the module docs - and this
    /// is what a renderer would wrap when there is.
    type Image = u32;

    fn presentation(&self) -> wxr::Presentation {
        wxr::Presentation::Composited
    }

    fn state(&self) -> wxr::State {
        self.state
    }

    fn poll(&mut self) -> Option<wxr::Event> {
        // Taken out before anything is written: a `JsValue` clone is a handle, and holding the borrow
        // across the writes below is what a `RefCell` refuses.
        let phase = match &*self.connect.borrow() {
            Connect::Pending => None,
            Connect::Failed(message) => Some(Err(message.clone())),
            Connect::Started(session) => Some(Ok(session.clone())),
        };
        match phase {
            None => None,
            Some(Err(message)) => {
                log::error!("wxr-webxr: the session was refused: {message}");
                self.state = wxr::State::Ended;
                Some(wxr::Event::Lost)
            }
            Some(Ok(session)) => {
                self.inner.borrow_mut().session = Some(session);
                self.state = wxr::State::Ready;
                self.request_frame();
                Some(wxr::Event::StateChanged(wxr::State::Ready))
            }
        }
    }

    fn images(&self) -> wxr::ImageMeta {
        // No image, and saying so with an empty extent rather than a plausible size: a renderer that reads
        // this and draws anyway has been told the truth.
        wxr::ImageMeta::default()
    }

    fn image_count(&self) -> usize {
        0
    }

    fn image(&self, _index: usize) -> Option<&Self::Image> {
        None
    }

    fn space(&mut self, kind: wxr::SpaceKind) -> Result<wxr::ReferenceSpace, wxr::Error> {
        let Some(session) = self.inner.borrow().session.clone() else {
            return Err(wxr::Error::Unavailable(
                "the session has not started yet".into(),
            ));
        };
        let slot = Rc::new(RefCell::new(None));
        let promise: js_sys::Promise = session
            .request_reference_space(reference_space_type(kind))
            .unchecked_into();

        let fill = slot.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(value) = JsFuture::from(promise).await
                && let Ok(space) = value.dyn_into::<XrReferenceSpace>()
            {
                *fill.borrow_mut() = Some(space);
            }
        });

        self.spaces.push(slot);
        Ok(wxr::ReferenceSpace::new(
            kind,
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
            .and_then(|slot| slot.borrow().clone())
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

        let views = pose.views();
        let out_views = out.views_mut();
        for index in 0..views.length() {
            let Ok(view) = views.get(index).dyn_into::<web_sys::XrView>() else {
                continue;
            };
            out_views.push(wxr::View {
                eye: match view.eye() {
                    web_sys::XrEye::Left => wxr::Eye::Left,
                    web_sys::XrEye::Right => wxr::Eye::Right,
                    _ => wxr::Eye::Mono,
                },
                pose: transform(view.transform()),
                fov: field_of_view(&view.projection_matrix()),
                viewport: wxr::Viewport::default(),
                image: 0,
                layer: index,
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
            .and_then(|slot| slot.borrow().clone())
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
        let sources = session.input_sources();
        for index in 0..sources.length() {
            let Some(source) = sources.get(index) else {
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
                handedness,
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

    fn end(&mut self, _frame: &mut wxr::Frame) -> Result<(), wxr::Error> {
        // There is nothing to hand back: the compositor has no image of ours. The next frame is asked for,
        // which is what keeps the session running.
        self.request_frame();
        Ok(())
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
