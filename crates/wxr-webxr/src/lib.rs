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
        let mut optional = vec![JsValue::from_str("webgpu")];
        // Surfaces are asked for in the session that is drawn over the world, which is the only kind that has
        // them - and as optional, because a browser that will not grant them is a session with no table in it
        // rather than no session.
        if mode == wxr::SessionMode::ImmersiveAr {
            // What the world-understanding modules need, and only in the session that has a world.
            optional.push(JsValue::from_str("plane-detection"));
            optional.push(JsValue::from_str("hit-test"));
            optional.push(JsValue::from_str("light-estimation"));
            optional.push(JsValue::from_str("depth-sensing"));
        }
        init.set_optional_features(&optional);
        // Depth arrives as an `ArrayBuffer` per view under `cpu-optimized`, which is the one a WebGPU session
        // can read without a renderer to import a texture - so that is what is asked for, in the format that
        // needs no unpacking. `depth-sensing` as a feature is not enough: the specification wants this key
        // beside it whenever the feature is granted.
        if mode == wxr::SessionMode::ImmersiveAr {
            let sensing = js_sys::Object::new();
            let _ = js_sys::Reflect::set(
                &sensing,
                &JsValue::from_str("usagePreference"),
                &js_sys::Array::of1(&JsValue::from_str("cpu-optimized")),
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
