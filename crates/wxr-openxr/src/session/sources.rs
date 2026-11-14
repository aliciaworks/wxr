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
        let Some(index) = self.space_index(space) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let reference = &self.spaces[index];
        let Some(hands) = self.hands.as_mut() else {
            return Ok(());
        };
        hands.read(&self.session, reference, self.predicted, out);
        Ok(())
    }
}

impl OpenXrSession {
    /// The profile names for one source: the eyes' fixed one, or what the runtime reports for a hand.
    ///
    /// OpenXR has one current interaction profile per user path rather than a list, and an unknown or absent
    /// profile is `Path::NULL` - a runtime that has none, which is the same empty answer a source this session
    /// did not hand out gets. The name is the runtime's own, the `/interaction_profiles/...` path that
    /// `xrPathToString` gives back, and not one of WebXR's short names.
    pub(super) fn profiles_impl(
        &mut self,
        source: wxr::InputId,
        out: &mut Vec<String>,
    ) -> Result<(), wxr::Error> {
        out.clear();
        let Some(hands) = self.hands.as_ref() else {
            return Ok(());
        };
        if let Some(profile) = hands.fixed_profile(source) {
            out.push(profile.to_owned());
            return Ok(());
        }
        let Some(user) = hands.user_path(source) else {
            return Ok(());
        };
        let profile = self
            .session
            .current_interaction_profile(user)
            .map_err(|error| wxr::Error::Present(format!("{error:?}")))?;
        if profile != xr::Path::NULL
            && let Ok(name) = self.instance.path_to_string(profile)
        {
            out.push(name);
        }
        Ok(())
    }

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
        let Some(index) = self.space_index(space) else {
            return Err(wxr::Error::NoSpace(space.kind));
        };
        let reference = &self.spaces[index];
        let Some(hands) = self.hands.as_ref() else {
            return Ok(());
        };
        hands.hand(source.get() as usize, reference, self.predicted, out);
        Ok(())
    }
}
