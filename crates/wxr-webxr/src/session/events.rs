//! The session's lifecycle, as events.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here.

use super::*;

impl WebXrSession {
    pub(super) fn poll_impl(&mut self) -> Option<wxr::Event> {
        // An ended session stays ended. `Lost` is the last thing it says - the session object may still be
        // sitting in `inner` for the browser to finish tearing down, and reading it as a live session on the
        // next poll would put the lifecycle back on `Ready` after it had already said it was gone.
        if self.state == wxr::State::Ended {
            if !self.lost {
                self.lost = true;
                return Some(wxr::Event::Lost);
            }
            return None;
        }

        // Taken out of the slot as it is read: a session that has arrived moves into the session, and a poll
        // that left it there would find it again every time - and ask the browser for another animation frame
        // every time with it, one closure per tick, none of them ever dropped.
        let arrived = match std::mem::replace(&mut *self.connect.borrow_mut(), Connect::Pending) {
            Connect::Started(session) => Some(session),
            Connect::Failed(message) => {
                log::error!("wxr-webxr: the session was refused: {message}");
                // A refusal is the same ending as a session that was granted and then went away: the state
                // first, and `Lost` from the guard at the top of the next poll. Reporting only `Lost` here
                // skipped the state every other path to the end reports, and set the flag that promises
                // `Lost` had a state to follow.
                self.state = wxr::State::Ended;
                return Some(wxr::Event::StateChanged(wxr::State::Ended));
            }
            Connect::Pending => None,
        };
        if let Some(session) = arrived {
            // The state is still `Connecting` here on purpose. The lifecycle section below is the one place
            // that compares it with what is now in the slot, and assigning `Ready` here would make that
            // comparison see no change - which is a caller that never hears the session arrived.
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

        // Before the visibility, because a session that has ended has no visibility worth reading. The
        // `Lost` that follows is the guard at the top of the next poll: a session says it ended, and then
        // says it is gone, in that order and only once.
        if self.ended.get() {
            self.state = wxr::State::Ended;
            return Some(wxr::Event::StateChanged(wxr::State::Ended));
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
