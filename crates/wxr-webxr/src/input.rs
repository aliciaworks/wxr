//! WebXR input: the sources, the identity an event names one by, and the six events.
//!
//! WebXR names an input source by the object it is and reports presses as events on the session, while a core
//! hands out an id and polls a queue. Both directions of that are the same translation, and it lives here so
//! that a frame and an event cannot disagree about which source they mean.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::{XrInputSource, XrInputSourceEvent, XrSession};

/// The sources a session has seen, in the order it first saw them.
///
/// The id it hands out is that order, and it is stable because the browser keeps returning the same object for
/// as long as a source exists - the one thing here that a value cannot carry as itself, since two sources
/// compared by value are two poses compared.
#[derive(Clone, Default)]
pub struct Sources {
    known: Rc<RefCell<Vec<XrInputSource>>>,
}

impl Sources {
    pub fn new() -> Self {
        Self::default()
    }

    /// The core's id for `source`, remembering it if it has not been seen before.
    pub fn id(&self, source: &XrInputSource) -> wxr::InputId {
        let mut known = self.known.borrow_mut();
        if let Some(index) = known.iter().position(|seen| seen == source) {
            return wxr::InputId::new(index as u32);
        }
        known.push(source.clone());
        wxr::InputId::new((known.len() - 1) as u32)
    }
}

/// How a source is aimed, read off the source itself.
///
/// `web-sys` binds `XrTargetRayMode` without the specification's fourth value, `transient-pointer`, so the
/// string is what is read - the way `environmentBlendMode` is read, and for the same reason.
pub fn target_ray_mode(source: &XrInputSource) -> wxr::TargetRayMode {
    let value = js_sys::Reflect::get(source.as_ref(), &JsValue::from_str("targetRayMode"));
    match value.ok().and_then(|value| value.as_string()).as_deref() {
        Some("gaze") => wxr::TargetRayMode::Gaze,
        Some("screen") => wxr::TargetRayMode::Screen,
        Some("transient-pointer") => wxr::TargetRayMode::TransientPointer,
        _ => wxr::TargetRayMode::TrackedPointer,
    }
}

/// The session's six input events, subscribed to and queued.
pub struct Events {
    queue: Rc<RefCell<VecDeque<wxr::Event>>>,
    /// The handlers, kept alive for as long as the session is: a closure the browser holds and this drops is a
    /// closure that stops being called. Nothing reads them - holding them *is* the work, which is what the
    /// allow says.
    #[allow(dead_code)]
    handlers: Vec<Closure<dyn FnMut(XrInputSourceEvent)>>,
}

impl Events {
    /// Subscribe to `selectstart`, `selectend`, `select`, `squeezestart`, `squeezeend` and `squeeze`.
    ///
    /// They are the same body with a different core event at the end, which is WebXR's own shape: three for the
    /// primary action, three for the grip, and the order is a start, an end, and the completion that follows
    /// the end.
    pub fn new(session: &XrSession, sources: &Sources) -> Self {
        let queue = Rc::new(RefCell::new(VecDeque::new()));
        let mut handlers = Vec::new();

        let handler = |make: fn(wxr::InputId) -> wxr::Event| {
            let sources = sources.clone();
            let queue = queue.clone();
            Closure::wrap(Box::new(move |event: XrInputSourceEvent| {
                queue
                    .borrow_mut()
                    .push_back(make(sources.id(&event.input_source())));
            }) as Box<dyn FnMut(XrInputSourceEvent)>)
        };

        let selectstart = handler(wxr::Event::SelectStart);
        session.set_onselectstart(Some(selectstart.as_ref().unchecked_ref()));
        handlers.push(selectstart);

        let selectend = handler(wxr::Event::SelectEnd);
        session.set_onselectend(Some(selectend.as_ref().unchecked_ref()));
        handlers.push(selectend);

        let select = handler(wxr::Event::Select);
        session.set_onselect(Some(select.as_ref().unchecked_ref()));
        handlers.push(select);

        let squeezestart = handler(wxr::Event::SqueezeStart);
        session.set_onsqueezestart(Some(squeezestart.as_ref().unchecked_ref()));
        handlers.push(squeezestart);

        let squeezeend = handler(wxr::Event::SqueezeEnd);
        session.set_onsqueezeend(Some(squeezeend.as_ref().unchecked_ref()));
        handlers.push(squeezeend);

        let squeeze = handler(wxr::Event::Squeeze);
        session.set_onsqueeze(Some(squeeze.as_ref().unchecked_ref()));
        handlers.push(squeeze);

        Self { queue, handlers }
    }

    /// The next event the browser delivered, oldest first.
    pub fn poll(&self) -> Option<wxr::Event> {
        self.queue.borrow_mut().pop_front()
    }
}
