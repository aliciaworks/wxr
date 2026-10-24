//! The session's own events, which are the runtime's news translated into the core's.
//!
//! Separate from the frame loop for the same reason the other backends separate theirs: polling is what a
//! frame loop does before it asks for a frame, and a state change or an input edge is not a thing the frame
//! itself contains. The frame loop's own answers live in `frames`.

use super::*;

impl OpenXrSession {
    pub(super) fn poll_impl(&mut self) -> Option<wxr::Event> {
        // The press edges the last frame produced come first: they are this crate's own, and waiting for the
        // runtime to get round to an instance event to report them would be a delay with no cause.
        if let Some(hands) = self.hands.as_mut()
            && let Some(event) = hands.poll()
        {
            return Some(event);
        }

        // Making a raycaster is asynchronous and a future only moves when something polls it, so this is the
        // one place it does - and this is the call a frame loop makes every frame anyway, which is why the
        // future was chosen to be finished here rather than needing a call of its own.
        if let Some(raycast) = self.raycast.as_mut() {
            raycast.poll();
        }

        // Events come off the instance, one at a time. A state change is the one a frame loop acts on and a
        // profile change is the set of inputs changing under it; the rest are the runtime's business, and
        // skipping them is better than inventing a mapping for them.
        loop {
            match self.instance.poll_event(&mut self.events) {
                // Which controllers the runtime has bound, which is the set of inputs changing: what they then
                // *are* is the next frame's answer, from `inputs`.
                Ok(Some(xr::Event::InteractionProfileChanged(_))) => {
                    return Some(wxr::Event::InputsChanged);
                }
                Ok(Some(xr::Event::SessionStateChanged(event))) => {
                    let openxr = event.state();
                    self.openxr_state = openxr;
                    // OpenXR's ladder is where both of the core's axes are read from, and the interesting
                    // rung is `VISIBLE` without `FOCUSED`: a session on a display that nobody is attending
                    // to, which is exactly WebXR's `visible-blurred`.
                    // `STOPPING` is the runtime asking the app to end the session rather than the session
                    // already being over: the one thing `ExitRequested` is for, and the app is expected to
                    // shut down cleanly rather than have it done for it. The session is still here, so it is
                    // still `Ready` - just not on a display any more.
                    if openxr == xr::SessionState::STOPPING {
                        self.state = wxr::State::Ready;
                        self.visibility = wxr::Visibility::Hidden;
                        return Some(wxr::Event::ExitRequested);
                    }
                    let (state, visibility) = match openxr {
                        xr::SessionState::READY => {
                            // A session runs only after the app has begun it, and only once it is ready -
                            // the runtime says `XR_ERROR_SESSION_NOT_RUNNING` from `xrWaitFrame` until then,
                            // which is the one thing it will not do on the app's behalf.
                            if !self.begun {
                                if let Err(error) = self
                                    .session
                                    .begin(xr::ViewConfigurationType::PRIMARY_STEREO)
                                {
                                    log::error!("wxr-openxr: beginning the session: {error:?}");
                                }
                                self.begun = true;
                            }
                            (wxr::State::Ready, wxr::Visibility::Hidden)
                        }
                        xr::SessionState::IDLE | xr::SessionState::SYNCHRONIZED => {
                            (wxr::State::Ready, wxr::Visibility::Hidden)
                        }
                        xr::SessionState::VISIBLE => {
                            (wxr::State::Ready, wxr::Visibility::VisibleBlurred)
                        }
                        xr::SessionState::FOCUSED => (wxr::State::Ready, wxr::Visibility::Visible),
                        xr::SessionState::LOSS_PENDING | xr::SessionState::EXITING => {
                            (wxr::State::Ended, wxr::Visibility::Hidden)
                        }
                        // A state this crate has not learned is not a state to guess at: the session is
                        // still whatever it was, and the next event will say what happened.
                        _ => continue,
                    };
                    if state != self.state {
                        self.state = state;
                        return Some(wxr::Event::StateChanged(state));
                    }
                    if visibility != self.visibility {
                        self.visibility = visibility;
                        return Some(wxr::Event::VisibilityChanged(visibility));
                    }
                    continue;
                }
                // An anchor the runtime has finished making, which is not a core event: `wxr::Event` has no
                // way to say "the anchor you asked for exists now", because the core's shape for that is the
                // `None` `anchor_pose` already answers until it does. This is the backend noticing, and the
                // loop carries on rather than returning - there is nothing for the caller to do about it.
                Ok(Some(xr::Event::SpatialAnchorCreateCompleteFB(event))) => {
                    if let Some(anchors) = self.anchors.as_mut() {
                        anchors.complete(event.request_id(), event.result(), event.space());
                    }
                    continue;
                }
                Ok(Some(_)) => continue,
                Ok(None) => {
                    // A session the runtime has taken away is `Lost` once, after the state that said so: a frame
                    // loop that keeps polling is told the difference between over and gone.
                    if self.state == wxr::State::Ended && !self.lost {
                        self.lost = true;
                        return Some(wxr::Event::Lost);
                    }
                    return None;
                }
                Err(error) => {
                    log::error!("wxr-openxr: polling events: {error:?}");
                    // A poll that fails is the instance going away, which is the same ending a state event
                    // would have said. Say the state first if it has not been said, then `Lost` once: a
                    // persistent error must not turn every later poll into another `Lost`, and a caller that
                    // only ever hears `Lost` never sees the state that explains it.
                    if self.state != wxr::State::Ended {
                        self.state = wxr::State::Ended;
                        return Some(wxr::Event::StateChanged(wxr::State::Ended));
                    }
                    if !self.lost {
                        self.lost = true;
                        return Some(wxr::Event::Lost);
                    }
                    return None;
                }
            }
        }
    }
}
