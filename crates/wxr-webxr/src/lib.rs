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
//! And images: this *does* hand them over, through WebXR's binding for WebGPU (`XRGPUBinding`), and only when
//! three things line up, because each of the three is off by default somewhere:
//!
//! * the browser has `XRGPUBinding` at all, which in Chromium is the `webxr-webgpu-binding` flag - a developer
//!   feature that has to be asked for, and one that *does* work on Linux even though the announcement named
//!   Windows and Android. `WebXr::gpu_binding()` is how to ask.
//! * the session grants the `webgpu` feature, which is why it is asked for as *optional* rather than required:
//!   a browser that will not grant it renders WebGL, and a required feature that is not there is a session that
//!   does not exist at all.
//! * the device came from an adapter requested with `xrCompatible: true`. This is the one wgpu does not have:
//!   the field has been proposed upstream and not landed, for being a breaking change to a public struct, which
//!   is why this workspace patches wgpu to a fork that carries it. Without the field, `XRGPUBinding`'s
//!   constructor throws, and this backend says so and carries on with no images.
//!
//! When any of them is missing, the session is `wxr::Presentation::Composited` with no images: it hands a
//! renderer the head, the eyes and the timing, and where the picture goes is the renderer's own canvas. Saying
//! that plainly is better than an `Image` type that is not one.
//!
//! Two things about the binding are worth writing down, because both are surprises. A WebGPU-compatible session
//! is **layers-only**: `baseLayer` must not be set, and a projection layer made through `XRGPUBinding` is what
//! a session needs - *without* a layer, `requestAnimationFrame` calls back zero times, which is the spec's
//! design and not a bug in anybody's code. And such a session reports projection matrices in a `0..w` clip
//! depth range instead of WebGL's `-w..w`, so the conversion a WebGL session needs is the wrong one for it.
//!
//! What the binding needs is three calls, and they are the three `gpu` and `session` make: `new
//! XRGPUBinding(session, device)`, `binding.createProjectionLayer({ colorFormat:
//! binding.getPreferredColorFormat() })`, and `session.updateRenderState({ layers: [layer] })` - after which a
//! frame's `binding.getViewSubImage(layer, view)` answers with the **same** colour and depth textures for both
//! eyes and a *per-view* texture view descriptor and viewport. That last part is the same shape the core
//! already carries: `View::viewport` is exactly what a sub-image reports, and the two eyes are views of one
//! texture rather than two textures.

#![cfg(target_family = "wasm")]

mod anchors;
mod convert;
mod depth;
mod gpu;
mod hit;
mod import;
mod input;
mod light;
mod planes;
mod session;

pub use import::Images;
pub use session::{FrameImage, WebXrSession};

/// The transform the world-understanding modules reach by name, kept at the root so a module can say
/// `crate::transform`.
pub(crate) use convert::transform;

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{XrSession, XrSessionMode};

use crate::convert::session_mode;
use crate::session::Connect;

/// What the renderer made.
///
/// The device is the whole of it, and that is not a simplification: `XRGPUBinding` is constructed from the
/// device, and there is nothing else of the renderer's a WebXR session wants. It has to be a device a session
/// will accept, though - one from an adapter requested with `xrCompatible: true`, which is the field this
/// workspace's wgpu fork carries and upstream does not. A renderer with any other device gets a session with
/// the head, the eyes and the timing and no images.
pub struct Device {
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
    /// answers on the same browser run twice. It says the *browser* could hand over images; whether a session
    /// does is the other two conditions - the session granting `webgpu`, and a device from an XR-compatible
    /// adapter - and both of those are things an app arranges rather than reads.
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
        let mut optional = vec![JsValue::from_str("webgpu")];
        // Surfaces are asked for in the session that is drawn over the world, which is the only kind that has
        // them - and as optional, because a browser that will not grant them is a session with no table in it
        // rather than no session.
        if mode != wxr::SessionMode::Inline {
            // Hands are an input mode in both immersive sessions, and a browser that will not grant them is a
            // session with controllers or nothing.
            optional.push(JsValue::from_str("hand-tracking"));
        }
        if mode == wxr::SessionMode::ImmersiveAr {
            // What the world-understanding modules need, and only in the session that has a world.
            optional.push(JsValue::from_str("plane-detection"));
            optional.push(JsValue::from_str("hit-test"));
            optional.push(JsValue::from_str("light-estimation"));
            optional.push(JsValue::from_str("depth-sensing"));
            optional.push(JsValue::from_str("anchors"));
        }
        init.set_optional_features(&optional);
        // Depth is asked for as `gpu-optimized`, which is the delivery that hands over a buffer rather than
        // bytes - the one the core's `Session::Depth` is shaped for, and the one a renderer can test against.
        // `depth-sensing` as a feature is not enough: the specification wants this key beside it whenever the
        // feature is granted.
        if mode == wxr::SessionMode::ImmersiveAr {
            let sensing = js_sys::Object::new();
            let _ = js_sys::Reflect::set(
                &sensing,
                &JsValue::from_str("usagePreference"),
                &js_sys::Array::of1(&JsValue::from_str("gpu-optimized")),
            );
            let _ = js_sys::Reflect::set(
                &sensing,
                &JsValue::from_str("dataFormatPreference"),
                &js_sys::Array::of1(&JsValue::from_str("float32")),
            );
            let _ = js_sys::Reflect::set(
                init.unchecked_ref::<JsValue>(),
                &JsValue::from_str("depthSensing"),
                &sensing,
            );
        }
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

impl From<Error> for wxr::Error {
    fn from(error: Error) -> Self {
        wxr::Error::Unavailable(error.to_string())
    }
}
