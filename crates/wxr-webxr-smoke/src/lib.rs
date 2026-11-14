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
//! `serve.py` beside this file builds it, binds `wasm-bindgen` to it, and serves it - and a desktop browser
//! has WebXR and no headset behind it, so it also serves `webxr-stub.js`: an XRSystem with a head, two hands
//! and two controllers, made here rather than borrowed from an extension. `drive.py` presses the button and
//! reads the verdict back off the server's record.
//!
//! **It runs twice, and the address is the difference.** `stub.html` is a session with no binding and no
//! pictures, which is what this backend is on a browser that will not hand an image over. `stub.html?gpu` is
//! the same page with the stub's `XRGPUBinding` in front of it, and that is the run with an `ImageMeta`, a
//! viewport and an array layer per eye, a `Features::DEPTH` bit and a measured depth to read back. Two runs of
//! one page because the thing under test is a code path, and a page cannot be in two states at once.

#![cfg(target_family = "wasm")]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wxr::{Backend as _, Session as _};

mod anchors;
mod hands;
mod images;
mod layers;
mod tick;

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
        if let Some(log) = window.document().as_ref().and_then(log_element) {
            let before = log.inner_text();
            log.set_inner_text(&format!("{before}{line}\n"));
        }
    }
}

/// Where the lines go on the page, made the first time one is printed.
///
/// An element of its own rather than the body, which is where this used to write: `set_inner_text` on a
/// node replaces *everything in it*, so the button that had just been appended to the body was gone by the
/// time the line saying it was waiting had been printed - and the run that needed it to be pressed found a
/// page with nothing to press.
fn log_element(document: &web_sys::Document) -> Option<web_sys::HtmlElement> {
    let element = match document.get_element_by_id("log") {
        Some(element) => element,
        None => {
            let element = document.create_element("pre").ok()?;
            element.set_id("log");
            document.body()?.append_child(&element).ok()?;
            element
        }
    };
    element.dyn_into().ok()
}

/// A run that did not draw, said once and in the one place a driver looks.
///
/// The smoke page has exactly two verdicts, `done` and `FAILED`, and both are lines on the record rather than
/// a status read out of the page: a driver that had to ask the DOM would be a driver that could be reading a
/// page that had not started yet.
fn fail(reason: &str) {
    say(&format!("FAILED: {reason}"));
}

/// Whether this page was opened at the address that asks for the stub's WebGPU binding.
///
/// The driver's decision and not this page's, which is the point: the same page is served twice and the only
/// difference between the two runs is the query string. Read here rather than passed in, because the page is
/// loaded once and clicked once and the address does not change in between.
fn opened_for_images() -> bool {
    web_sys::window()
        .and_then(|window| window.location().search().ok())
        .is_some_and(|search| search.contains("gpu"))
}

/// What the stub's `XRView.requestViewportScale` last took, if it took anything.
///
/// The browser's answer to a request is not the request, and a view is the session's rather than the page's,
/// so the stub records what it accepted where the page can read it. `None` before the first request.
fn viewport_scale() -> Option<f64> {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("__wxrViewportScale"))
        .ok()
        .and_then(|value| value.as_f64())
}

/// What the stub's `XRSession.updateTargetFrameRate` last took, if it took anything.
fn frame_rate() -> Option<f64> {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("__wxrFrameRate"))
        .ok()
        .and_then(|value| value.as_f64())
}

/// How many times `XRSession.updateTargetFrameRate` was called at all, taken or refused.
fn frame_rate_requests() -> Option<u32> {
    js_sys::Reflect::get(
        &js_sys::global(),
        &JsValue::from_str("__wxrFrameRateRequests"),
    )
    .ok()
    .and_then(|value| value.as_f64())
    .map(|count| count as u32)
}

/// What one run is holding: the frame being built, and how far it has got.
struct Board {
    frame: wxr::Frame,
    space: Option<wxr::ReferenceSpace>,
    /// A space the browser refuses, asked for alongside the floor: the stub has no `unbounded`, and a request
    /// that settles without a space is the one shape an offset has to refuse rather than wait on forever.
    unbounded: Option<wxr::ReferenceSpace>,
    /// Whether that refusal has been checked, which is once - after the promise has had a frame to settle.
    refusal_checked: bool,
    drawn: u32,
    ticks: u32,
    /// Set once the run has said its verdict, so the last requested frame draws nothing.
    stopped: bool,
    /// The last lifecycle reported, and the last visibility, so a rung of either ladder is news once.
    state: wxr::State,
    visibility: wxr::Visibility,
    /// Whether the inputs have been reported, so a controller is news once rather than every frame.
    input_said: bool,
    /// Whether the skeletons have been asked for, and the buzzes sent, for the same reason.
    hands_said: bool,
    /// The hands, because a joint is asked of one and not of a session.
    hands: Vec<wxr::Hand>,
    /// Whether this run asked for the binding, which is what says whether images are promised at all.
    gpu: bool,
    /// Whether the images have been asserted, which is once: every frame of a working run is the same answer,
    /// and a run whose eighth frame differs from its first has already failed.
    checked: bool,
    /// Whether a `views` call the browser cannot answer has been retried, which is once and after the image
    /// checks: it is the retry that must leave no picture behind, and the frame it checks is the one with eyes.
    retry_checked: bool,
    /// Whether the viewport scale has been asked for, which is once: the request applies to the frame it is
    /// made in, and the values that matter are the ones the browser cannot convert.
    viewport_scale_checked: bool,
    /// Whether the display's refresh rates have been read and asked for, which is once: the list is the
    /// session's, and the rate a request leaves behind is the stub's record.
    refresh_rate_checked: bool,
    /// Whether the layer count the session names has been read, which is once and needs no frame: the number
    /// is the session's and the stub says four.
    max_layers_checked: bool,
    /// The tick the request was made on, so the event that follows it is checked on the next poll and not on
    /// the tick that asked: a handler fires synchronously, and the poll that reports it is one tick later.
    refresh_rate_asked: u32,
    /// The rate the display announced changing to, if it has, and whether that has been checked.
    refresh_rate_event: Option<f32>,
    refresh_rate_event_checked: bool,
    /// How many rate-change events the session has reported, so a refused request cannot slip a second one in.
    refresh_rate_events: u32,
    /// Whether a space handle this session never made has been asked about, which is once: the answer is the
    /// same on every frame, and the call writes only the vector it is given.
    forged_space_checked: bool,
    /// The viewer space, kept because the refusal of an environment layer in it is checked once and a space
    /// made twice is two slots for one answer.
    viewer: Option<wxr::ReferenceSpace>,
    /// Whether the layers that must be refused have been, which is once and before any is made.
    layers_refused: bool,
    /// Whether the four shapes have been handed over, which happens between frames: the timer makes them, and
    /// the frame loop only reads them back.
    layers_prepared: bool,
    /// Whether the layers have been read back in a frame, which is once and after they were made.
    layers_checked: bool,
    /// The layer the compositor asked to have drawn again, if it has asked, and whether that was checked.
    redraw_event: Option<wxr::Layer>,
    redraw_checked: bool,
    /// The layers the compositor was given, in the order they were made.
    layers: Vec<wxr::Layer>,
    /// The anchor this run made, if it has, and how far through the checks it is: zero until it asks, and
    /// four once the released anchor has been seen to be gone.
    anchor: Option<wxr::Anchor>,
    /// The name the browser gave the anchor, which the restore checks ask for again.
    anchor_name: Option<wxr::AnchorId>,
    anchor_step: u8,
    /// Whether asking for an anchor with no frame open has been refused, which is once and before any frame.
    anchor_unframed_checked: bool,
    /// Whether the floor space has resolved. A space is a promise, so the tick that asks for one has no pose
    /// to make an anchor at - and an anchor asked for in an unresolved space is a different refusal.
    space_resolved: bool,
}

/// Record the events that are checked later rather than as they arrive.
///
/// Both pollers - the timer that waits for the session and the frame loop - take events off the same queue,
/// and only one of them sees any given event, so the note is taken here rather than in either loop.
fn note_event(board: &mut Board, event: wxr::Event) {
    if let wxr::Event::RefreshRateChanged(rate) = event {
        board.refresh_rate_event = Some(rate);
        board.refresh_rate_events += 1;
    }
    if let wxr::Event::Redraw(layer) = event {
        board.redraw_event = Some(layer);
    }
}

/// The backend's own log lines, which are the ones that say *why*.
///
/// A refused session arrives as an event, and an event has nowhere to put a sentence - so `wxr-webxr` logs
/// the browser's own explanation and hands back `Lost`. A page with no logger drops those lines on the floor,
/// and what is left on the record is "the session was lost" with no reason in it, which is the least useful
/// sentence a backend can produce.
struct Beacon;

impl log::Log for Beacon {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        say(&record.args().to_string());
    }

    fn flush(&self) {}
}

#[wasm_bindgen(start)]
fn start() {
    // Before anything else, so that the backend's explanation of a failure is on the record before the
    // failure is. A second logger is not an error worth stopping for: there is only ever one page here.
    if log::set_logger(&Beacon).is_ok() {
        log::set_max_level(log::LevelFilter::Trace);
    }
    wasm_bindgen_futures::spawn_local(setup());
}

async fn setup() {
    say("wxr-webxr smoke: starting");
    say(&format!(
        "opened at {}",
        if opened_for_images() {
            "the address that asks for the stub's WebGPU binding"
        } else {
            "the plain address, which is a session with no images to give"
        }
    ));

    // Asked here as well as in the click handler, because a browser with no XR system at all should say so
    // before it asks for a button rather than after somebody presses one. There is nothing to keep: `WebXr`
    // is a handle on `navigator.xr`, and the one that matters is the one current when the gesture happens.
    if let Err(error) = wxr_webxr::WebXr::load() {
        return fail(&format!("no WebXR here: {error}"));
    }
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
        Err(error) => return fail(&format!("no adapter: {error:?}")),
    };
    let (device, _queue) = match adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
    {
        Ok(pair) => pair,
        Err(error) => return fail(&format!("no device: {error:?}")),
    };
    say(&format!(
        "a WebGPU device, made by the page (asked for an XR-compatible adapter: {xr_compatible})"
    ));

    // A button, and not a call on load. `requestSession` is one of the calls the specification reserves for a
    // user gesture, so a page that asks as soon as it loads gets `SecurityError` and a session that never
    // exists - which is the platform's rule and not this page's shortcoming. Every immersive page ever written
    // has a button in it for exactly this, and a page without one could only ever fail here.
    let document = web_sys::window()
        .and_then(|window| window.document())
        .expect("a page has a document");
    let button = document
        .create_element("button")
        .expect("a document can make an element");
    button.set_id("enter");
    button.set_text_content(Some("Enter XR"));
    if let Some(body) = document.body() {
        // Before the log rather than after it: a page whose button is a screenful down is a page a driver has
        // to scroll, and coordinates are the only thing a driver has to aim with.
        let _ = body.insert_before(&button, body.first_child().as_ref());
    }
    say("waiting for a click to enter XR");

    let on_click = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        // The backend again rather than a copy of one: `WebXr` is a handle on `navigator.xr`, and holding it
        // across a click would be holding something the page is free to have replaced in the meantime.
        let backend = match wxr_webxr::WebXr::load() {
            Ok(backend) => backend,
            Err(error) => return fail(&format!("no WebXR here: {error}")),
        };
        let device = device.clone();
        wasm_bindgen_futures::spawn_local(async move { enter(backend, device).await });
    });
    let _ = button.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref());
    // The closure lives for as long as the page does, which is the only lifetime a page has.
    on_click.forget();
}

/// Everything after the gesture: the session, and the loop that draws its frames.
async fn enter(backend: wxr_webxr::WebXr, device: wgpu::Device) {
    let gpu = opened_for_images();
    // Two modes, and the reason is a feature rather than a preference: `depth-sensing` is asked for in an
    // immersive-ar session and only there, so a run that has a binding is the run that can measure the room.
    // A run that does not is immersive-vr, which is the mode with nothing to measure.
    let mode = if gpu {
        wxr::SessionMode::ImmersiveAr
    } else {
        wxr::SessionMode::ImmersiveVr
    };
    let session = match backend.connect(wxr_webxr::Device { device }, mode) {
        Ok(session) => session,
        Err(error) => return fail(&format!("connect refused: {error}")),
    };
    say(&format!(
        "connected as {mode:?}, state {:?}, visibility {:?}, presentation {:?}",
        session.state(),
        session.visibility(),
        session.presentation()
    ));

    // The handler lives inside the session and must reach it, so the session is shared and the handler
    // holds a `Weak` to break the cycle.
    let session = Rc::new(RefCell::new(session));
    let board = Rc::new(RefCell::new(Board {
        frame: wxr::Frame::default(),
        space: None,
        unbounded: None,
        refusal_checked: false,
        drawn: 0,
        ticks: 0,
        state: wxr::State::default(),
        visibility: wxr::Visibility::default(),
        input_said: false,
        hands_said: false,
        hands: Vec::new(),
        gpu,
        checked: false,
        retry_checked: false,
        viewport_scale_checked: false,
        refresh_rate_checked: false,
        max_layers_checked: false,
        refresh_rate_asked: 0,
        refresh_rate_event: None,
        refresh_rate_event_checked: false,
        refresh_rate_events: 0,
        forged_space_checked: false,
        viewer: None,
        layers_refused: false,
        // A run without a binding has no picture to place, so it has nothing to prepare and nothing to check.
        layers_prepared: !gpu,
        layers_checked: !gpu,
        redraw_event: None,
        redraw_checked: !gpu,
        layers: Vec::new(),
        anchor: None,
        anchor_name: None,
        anchor_step: 0,
        anchor_unframed_checked: false,
        space_resolved: false,
        stopped: false,
    }));

    // The loop runs inside the animation callback, because an `XRFrame` is only active while the callback
    // that was given it runs. Begin, draw and end all happen here.
    let weak_session = Rc::downgrade(&session);
    let loop_board = board.clone();
    let handler: Box<dyn FnMut()> = Box::new(move || {
        let Some(session) = weak_session.upgrade() else {
            return;
        };
        let mut session = session.borrow_mut();
        let mut board = loop_board.borrow_mut();
        if board.stopped {
            return;
        }
        // One verdict, and it is a line rather than a status: a driver that had to read the page would be a
        // driver reading a page that might not have started yet.
        let verdict = match tick::tick(&mut board, &mut session) {
            // The anchors are last because they are the newest: a run that drew its frames without them has
            // not exercised the anchor path, and a verdict that said `done` would be one that never did.
            Ok(())
                if board.drawn >= FRAMES
                    && board.layers_checked
                    && board.redraw_checked
                    && (!session.features().contains(wxr::Features::ANCHORS)
                        || board.anchor_step == 8) =>
            {
                Some("done".to_string())
            }
            Ok(()) if !session.state().is_alive() => Some(format!(
                "FAILED: the session ended after {} frame(s)",
                board.drawn
            )),
            Ok(()) => None,
            Err(reason) => Some(format!("FAILED: {reason}")),
        };
        let Some(verdict) = verdict else {
            return;
        };
        say(&verdict);
        // The frame already requested is the last this handler sees; a stopped handler draws nothing and
        // asks for no more frames.
        board.stopped = true;
    });
    session.borrow_mut().set_frame_handler(handler);

    // A session arrives by promise and `poll` is what takes it, so there is no animation frame to ask for
    // until then. This timer is only that pump: once the session is ready the handler owns the loop. The
    // closure is leaked so its `Rc` keeps the session - and through its handler, the board - alive.
    let ticking = session.clone();
    let interval: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
    let stop = interval.clone();
    let pumping = board.clone();
    let pump = Closure::<dyn FnMut()>::new(move || {
        let mut session = ticking.borrow_mut();
        while let Some(event) = session.poll() {
            say(&format!("event: {event:?}"));
            note_event(&mut pumping.borrow_mut(), event);
        }
        if !session.state().is_alive() {
            say(&format!(
                "FAILED: the session ended before it was ready ({:?})",
                session.state()
            ));
        } else {
            // A layer is handed over between frames and only then, so this is where it happens: the timer is
            // outside every animation frame by construction, where the frame loop is inside one.
            let mut board = pumping.borrow_mut();
            match layers::prepare(&mut board, &mut session) {
                Ok(true) => {}
                Ok(false) => return,
                Err(reason) => say(&format!("FAILED: {reason}")),
            }
        }
        if let (Some(id), Some(window)) = (stop.borrow_mut().take(), web_sys::window()) {
            window.clear_interval_with_handle(id);
        }
    });
    let handle = web_sys::window()
        .and_then(|window| {
            window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    pump.as_ref().unchecked_ref(),
                    16,
                )
                .ok()
        })
        .unwrap_or(0);
    pump.forget();
    *interval.borrow_mut() = Some(handle);
}

#[cfg(not(target_family = "wasm"))]
compile_error!("the WebXR smoke page is wasm only, like the backend it runs");
