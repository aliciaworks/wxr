//! The session's lifecycle, as events.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here.

use super::*;

impl WebXrSession {
    pub(super) fn poll_impl(&mut self) -> Option<wxr::Event> {
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
            // whose animation frames never arrive at all - and before that, the session is put where the layer
            // list can find it, because handing over a layer is a thing done *to* a session.
            self.inner.borrow_mut().session = Some(session.clone());
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
}
