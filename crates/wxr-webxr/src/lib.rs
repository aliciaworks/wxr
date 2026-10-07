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

// The generated bindings name `alloc::string::String` rather than `std`'s, because they are generated as if
// for a crate that may not have `std` - which is how `web-sys` is built.
extern crate alloc;

// The whole WebXR API, generated from its IDL rather than taken from `web-sys` - which has the core, gates it
// behind a build-wide cfg, and has none of the Layers module or the WebGPU binding at all. Generated and
// committed, the way wgpu carries `webgpu_sys`; `Tools/refresh_webxr_idl.py` is how it is refreshed.
pub mod sys;

mod anchors;
mod convert;
mod depth;
mod hit;
mod import;
mod input;
mod layers;
mod light;
mod planes;
mod session;
mod throws;

pub use import::Images;
pub use session::{FrameImage, WebXrSession};

/// The transform the world-understanding modules reach by name, kept at the root so a module can say
/// `crate::transform`.
pub(crate) use convert::transform;

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::convert::session_mode;
use crate::session::Connect;
use crate::sys::{XrDepthStateInit, XrSession, XrSessionInit, XrSessionMode, XrSystem};

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
    system: XrSystem,
}

impl WebXr {
    /// The `navigator.xr` of the page. A browser without WebXR is
    /// [`wxr::Error::Unavailable`] rather than a panic, and this is deliberately not "is a headset
    /// connected": that is a question for [`WebXr::is_supported`].
    pub fn load() -> Result<Self, Error> {
        let navigator = web_sys::window().ok_or(Error::NoWindow)?.navigator();
        // Read by name, and not through `web-sys`: `Navigator.xr` is generated here rather than taken from
        // there, which is the whole point of the generated module - one WebXR surface, in one place, with no
        // build-wide cfg under it. What a browser without WebXR hands back is an `undefined` where the object
        // should be, which is the question asked of the value.
        match js_sys::Reflect::get(&navigator, &JsValue::from_str("xr")) {
            Ok(value) if !value.is_undefined() && !value.is_null() => Ok(Self {
                system: value.unchecked_into(),
            }),
            _ => Err(Error::NoXr),
        }
    }

    /// Whether the browser can start a session of this kind, which is an async question with no synchronous
    /// answer - the promise is the answer.
    pub fn is_supported(&self, mode: XrSessionMode) -> js_sys::Promise<js_sys::Boolean> {
        // The generated binding leaves the promise untyped, because WebXR's IDL does: what it resolves to is
        // the caller's to know.
        self.system.is_session_supported(mode).unchecked_into()
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

/// A feature list as the browser takes it: an array of strings, which is what a `FrozenArray<DOMString>` is on
/// the way in as well as on the way out.
fn features(names: &[&str]) -> JsValue {
    let array = js_sys::Array::new();
    for name in names {
        array.push(&JsValue::from_str(name));
    }
    array.into()
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
        let init = XrSessionInit::new();
        // A floor is asked for only where there is a room to have one: an inline session has no floor, and a
        // required feature a runtime will not grant is a session that does not exist at all.
        if mode != wxr::SessionMode::Inline {
            init.set_required_features(&features(&["local-floor"]));
        }
        // `webgpu` is asked for as an *optional* feature: a browser that will not grant it is a browser that
        // renders WebGL, and a required feature that is not there is a session that does not exist at all. What
        // came back is what `Session::features` asks about before any of the binding is attempted.
        let mut optional = vec!["webgpu"];
        // A session that draws over the world is also the one that has hands, a room and the modules that read
        // it - and every one of them is optional, because a browser that will not grant one is a session
        // without that thing rather than no session.
        if mode != wxr::SessionMode::Inline {
            optional.push("hand-tracking");
            // Non-projection layers need the feature descriptor as well as the binding: a session can have a
            // WebGPU binding and still refuse to composite a quad, and asking is how that is found out.
            optional.push("layers");
        }
        if mode == wxr::SessionMode::ImmersiveAr {
            // What the world-understanding modules need, and only in the session that has a world.
            optional.push("plane-detection");
            optional.push("hit-test");
            optional.push("light-estimation");
            optional.push("depth-sensing");
            optional.push("anchors");
        }
        init.set_optional_features(&features(&optional));
        // Depth is asked for as `gpu-optimized`, which is the delivery that hands over a texture rather than
        // bytes - the one the core's `Session::Depth` is shaped for, and the one a renderer can test against.
        // `depth-sensing` as a feature is not enough: the specification wants this key beside it whenever the
        // feature is granted.
        if mode == wxr::SessionMode::ImmersiveAr {
            init.set_depth_sensing(&XrDepthStateInit::new(
                &features(&["float32"]),
                &features(&["gpu-optimized"]),
            ));
        }
        let requested: js_sys::Promise = self
            .system
            .request_session_with_options(session_mode(mode), &init);

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
