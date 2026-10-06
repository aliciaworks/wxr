//! A page that runs `wxr-webxr`, because a backend nobody has run is a backend nobody has seen work.
//!
//! The other two backends have runnable things: `wxr-openxr` has `examples/headless.rs`, and `wxr-apple` has
//! `entry::run`, which an app's `CompositorLayer` calls. This is the third one, and it is a crate of its own
//! rather than an example because `wasm-bindgen` needs a `cdylib` and an example cannot be one.
//!
//! It does what an app would do, in the order the core asks for it: load the browser's XR system, make a
//! WebGPU device, connect (which is `Connecting` until the promise lands), ask for a floor space once the
//! session can render, and run frames until it has drawn a few. Everything it learns it writes into the page
//! as well as the console, because a test whose result needs another window opened is a test nobody reads.
//!
//! `serve.py` beside this file builds it, binds `wasm-bindgen` to it, and serves it - and a browser needs a
//! WebXR device to hand over, which on a desktop means the Immersive Web Emulator extension loaded with
//! `--load-extension`.

#![cfg(target_family = "wasm")]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wxr::{Backend as _, Session as _};

/// How many frames to draw before saying so and stopping.
const FRAMES: u32 = 8;

/// One line, on the page, in the console, and at the server that served it.
///
/// The server is the one that matters: a page's console needs a window opened at the right moment to be read,
/// and whatever drives the browser is another moving part between a test and its result. A beacon is
/// fire-and-forget and has no promise to wait for.
fn say(line: &str) {
    web_sys::console::log_1(&JsValue::from_str(line));
    if let Some(window) = web_sys::window() {
        // Fire and forget: the promise is not awaited, because a log line is not worth a frame.
        let init = web_sys::RequestInit::new();
        init.set_method("POST");
        init.set_body(&JsValue::from_str(line));
        let _ = window.fetch_with_str_and_init("/log", &init);
        if let Some(body) = window.document().and_then(|document| document.body()) {
            let before = body.inner_text();
            body.set_inner_text(&format!("{before}{line}\n"));
        }
    }
}

/// What one run is holding: the session, the frame being built, and how far it has got.
struct Board {
    session: wxr_webxr::WebXrSession,
    frame: wxr::Frame,
    space: Option<wxr::ReferenceSpace>,
    drawn: u32,
    ticks: u32,
    /// The last lifecycle reported, and the last visibility, so a rung of either ladder is news once.
    state: wxr::State,
    visibility: wxr::Visibility,
    /// Whether the inputs have been reported, so a controller is news once rather than every frame.
    input_said: bool,
}

#[wasm_bindgen(start)]
fn start() {
    wasm_bindgen_futures::spawn_local(setup());
}

async fn setup() {
    say("wxr-webxr smoke: starting");

    let backend = match wxr_webxr::WebXr::load() {
        Ok(backend) => backend,
        Err(error) => return say(&format!("no WebXR here: {error}")),
    };
    say(&format!(
        "browser has the WebXR/WebGPU binding: {}",
        wxr_webxr::WebXr::gpu_binding()
    ));

    // The device, which the core gives every backend: on this platform it is the page's WebGPU device, and
    // the session is told about it like every other backend's is.
    //
    // `xr_compatible` is asked for exactly when the browser has a binding that will demand it, and it has to
    // be decided here: WebGPU has no `makeXRCompatible`, so an adapter requested without it can never make a
    // device for `XRGPUBinding`, and asking afterwards does not help.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let xr_compatible = wxr_webxr::WebXr::gpu_binding();
    let adapter = match instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            xr_compatible,
            ..Default::default()
        })
        .await
    {
        Ok(adapter) => adapter,
        Err(error) => return say(&format!("no adapter: {error:?}")),
    };
    let (device, _queue) = match adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
    {
        Ok(pair) => pair,
        Err(error) => return say(&format!("no device: {error:?}")),
    };
    say(&format!(
        "a WebGPU device, made by the page (asked for an XR-compatible adapter: {xr_compatible})"
    ));

    let session = match backend.connect(wxr_webxr::Device { instance, device }) {
        Ok(session) => session,
        Err(error) => return say(&format!("connect refused: {error}")),
    };
    say(&format!(
        "connected, state {:?}, visibility {:?}",
        session.state(),
        session.visibility()
    ));

    // The loop is the browser's, because WebXR has no blocking wait: the session's animation callback puts a
    // frame in a slot and this ticks over it, which is what an app does too.
    let board = Rc::new(RefCell::new(Board {
        session,
        frame: wxr::Frame::default(),
        space: None,
        drawn: 0,
        ticks: 0,
        state: wxr::State::default(),
        visibility: wxr::Visibility::default(),
        input_said: false,
    }));
    let ticking = board.clone();
    let interval: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
    let stop = interval.clone();
    let callback = Closure::<dyn FnMut()>::new(move || {
        let mut board = ticking.borrow_mut();
        tick(&mut board);
        if board.drawn >= FRAMES || !board.session.state().is_alive() {
            say("done");
            if let (Some(id), Some(window)) = (stop.borrow_mut().take(), web_sys::window()) {
                window.clear_interval_with_handle(id);
            }
        }
    });
    let handle = web_sys::window()
        .and_then(|window| {
            window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    callback.as_ref().unchecked_ref(),
                    16,
                )
                .ok()
        })
        .unwrap_or(0);
    // The closure lives for as long as the page does, which is the only lifetime a page has.
    callback.forget();
    *interval.borrow_mut() = Some(handle);
}

/// One tick of the loop: what happened, where the eyes are, and a frame handed back.
fn tick(board: &mut Board) {
    board.ticks += 1;
    while let Some(event) = board.session.poll() {
        say(&format!("event: {event:?}"));
    }
    // Reported on every change rather than per frame: a session that never reaches `Visible` is exactly the
    // case this backend has to be honest about, and frames never arrive to say it in. The lifecycle and the
    // visibility are separate axes, so each is reported when it moves.
    if board.session.state() != board.state || board.session.visibility() != board.visibility {
        board.state = board.session.state();
        board.visibility = board.session.visibility();
        let meta = board.session.images();
        say(&format!(
            "state {:?}, visibility {:?}: {} image(s), {:?} at {:?}",
            board.state,
            board.visibility,
            board.session.image_count(),
            meta.format,
            meta.extent
        ));
    }

    if board.space.is_none() && board.session.visibility().can_render() {
        match board.session.space(wxr::SpaceKind::LocalFloor) {
            Ok(space) => {
                say("asked for a floor space");
                board.space = Some(space);
            }
            Err(error) => {
                say(&format!("no floor space: {error}"));
                return;
            }
        }
    }
    let Some(space) = board.space else {
        return;
    };

    let now = Duration::from_millis(u64::from(board.ticks) * 16);
    if let Err(error) = board.session.begin(now, &mut board.frame) {
        return say(&format!("begin failed: {error}"));
    }
    if !board.frame.is_render() {
        return;
    }
    if let Err(error) = board.session.views(space, &mut board.frame) {
        say(&format!("views failed: {error}"));
    }
    for view in board.frame.views() {
        say(&format!(
            "  {:?} eye at ({:.3}, {:.3}, {:.3}), fov {:.3}/{:.3}/{:.3}/{:.3}, layer {} of image {}",
            view.eye,
            view.pose.position.x,
            view.pose.position.y,
            view.pose.position.z,
            view.fov.up,
            view.fov.down,
            view.fov.left,
            view.fov.right,
            view.layer,
            view.image,
        ));
    }
    // The sources, which are the frame's half of input: where the hands are and how they are aimed. What they
    // *do* arrives as events above, because that is where WebXR puts it.
    let mut sources = Vec::new();
    if let Err(error) = board.session.inputs(space, &mut sources) {
        say(&format!("inputs failed: {error}"));
    }
    if !board.input_said && !sources.is_empty() {
        board.input_said = true;
        for source in &sources {
            say(&format!(
                "  {:?} hand, {:?}, id {}, aiming from ({:.3}, {:.3}, {:.3})",
                source.handedness,
                source.target_ray_mode,
                source.id.get(),
                source.aim.position.x,
                source.aim.position.y,
                source.aim.position.z,
            ));
        }
    }

    if let Err(error) = board.session.end(&mut board.frame) {
        say(&format!("end failed: {error}"));
    }
    board.drawn += 1;
    say(&format!(
        "frame {} drawn ({} image(s) this session hands over)",
        board.drawn,
        board.session.image_count()
    ));
}

#[cfg(not(target_family = "wasm"))]
compile_error!("the WebXR smoke page is wasm only, like the backend it runs");
