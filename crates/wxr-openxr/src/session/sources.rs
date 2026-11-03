//! The input sources and the hands.
//!
//! Delegated from `wxr::Session`, which gets one implementation per type: the bodies live here and the
//! trait is the list of what is answered.

use super::*;

impl OpenXrSession {
    pub(super) fn inputs_impl(
        &mut self,
        space: wxr::ReferenceSpace,
        out: &mut Vec<wxr::InputSource>,
    ) -> Result<(), wxr::Error> {
        // Before the early return below: a session with no input extension has an empty list to report, not
        // whatever the caller was holding when it asked.
        out.clear();
        // The space before the input extension: a handle this session never made is refused whether or not
        // there are hands to read with it, which is the same answer `views` gives. A session with no hands
        // still has spaces of its own, and still answers with no sources for them.
        let Some(reference) = self.spaces.get(space.id() as usize) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let Some(hands) = self.hands.as_mut() else {
            return Ok(());
        };
        hands.read(&self.session, reference, self.predicted, out);
        Ok(())
    }
}

impl OpenXrSession {
    pub(super) fn hand_impl(
        &mut self,
        source: wxr::InputId,
        space: wxr::ReferenceSpace,
        out: &mut wxr::Hand,
    ) -> Result<(), wxr::Error> {
        // Before the space lookup, because that is the one path out of here that has no skeleton to write:
        // a hand refused for a space this session never made would otherwise leave the last frame's joints in
        // the caller's hand, which is a hand drawn where the runtime never said it was. `Hands::hand` clears
        // too, but only once it has been reached.
        out.clear();
        // The space first, for the reason `inputs` gives.
        let Some(reference) = self.spaces.get(space.id() as usize) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let Some(hands) = self.hands.as_ref() else {
            return Ok(());
        };
        hands.hand(source.get() as usize, reference, self.predicted, out);
        Ok(())
    }
}
