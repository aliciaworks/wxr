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
        let Some(hands) = self.hands.as_mut() else {
            return Ok(());
        };
        let Some(reference) = self.spaces.get(space.id() as usize) else {
            return Err(wxr::Error::NoSpace(space.kind));
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
        let Some(hands) = self.hands.as_ref() else {
            out.clear();
            return Ok(());
        };
        let Some(reference) = self.spaces.get(space.id() as usize) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        hands.hand(source.get() as usize, reference, self.predicted, out);
        Ok(())
    }
}
