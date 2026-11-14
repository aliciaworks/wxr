//! Leaving the session, from the app's side.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the body lives here.

use super::*;

impl WebXrSession {
    /// End the browser's session, which is `XRSession.end`.
    ///
    /// The promise is dropped rather than awaited, and that is the whole of the difference from `Drop`: what the
    /// caller asked for is the request, and the ending itself arrives as the `end` event the poll reports - a
    /// browser that refuses (a session that ended between the check and the call) leaves a rejection nothing is
    /// waiting on, which is worth a debug line rather than an error the caller cannot act on.
    pub(super) fn request_exit_impl(&mut self) -> Result<(), wxr::Error> {
        let Some(session) = self.inner.borrow().session.clone() else {
            // Nothing was ever granted, so there is nothing to end - and saying so is better than an `Ok` that
            // claims a request was made to a browser this session never reached.
            return Err(wxr::Error::Unavailable(
                "the session has not started yet".into(),
            ));
        };
        let promise =
            throws::end(&session).map_err(|error| wxr::Error::Rejected(format!("{error:?}")))?;
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(error) = JsFuture::from(promise).await {
                log::debug!("wxr-webxr: the browser would not end the session: {error:?}");
            }
        });
        Ok(())
    }
}
