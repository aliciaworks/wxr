//! The display's own knobs: how fast it runs, and how much of each frame it can fill.
//!
//! Both are requests rather than promises - a runtime that cannot do either keeps what it has - and both
//! are vendor extensions whose concepts the core carries as capability bits.

use super::*;

impl OpenXrSession {
    pub(super) fn set_foveation_impl(&mut self, amount: f32) {
        // `XR_FB_foveation` has four levels rather than a fraction, so an amount becomes the nearest of them.
        // A runtime that does not list the extension is a runtime that does not foveate, and a profile is
        // where that is found out - which is a knob left alone rather than a frame to fail.
        let level = match amount.clamp(0.0, 1.0) {
            a if a <= 0.0 => xr::FoveationLevelFB::NONE,
            a if a < 0.34 => xr::FoveationLevelFB::LOW,
            a if a < 0.67 => xr::FoveationLevelFB::MEDIUM,
            _ => xr::FoveationLevelFB::HIGH,
        };
        let profile = match self
            .session
            .create_foveation_profile(Some(xr::FoveationLevelProfile {
                level,
                vertical_offset: 0.0,
                dynamic: xr::FoveationDynamicFB::DISABLED,
            })) {
            Ok(profile) => profile,
            Err(error) => {
                log::debug!("wxr-openxr: no foveation for {amount}: {error:?}");
                return;
            }
        };

        // The typed crate makes a profile and has no way to put one on a swapchain, so this is the one raw call
        // in this backend: `xrUpdateSwapchainFB` from `XR_FB_swapchain_update_state`, which is what a profile is
        // for.
        let Some(update) = self.instance.exts().fb_swapchain_update_state.as_ref() else {
            return;
        };
        // The typed crate makes a profile and keeps the swapchain state that carries it private, so this is the
        // one raw structure in this backend. Zeroed is the whole of what it starts as: empty flags, no chain.
        let mut state: openxr_sys::SwapchainStateFoveationFB = unsafe { std::mem::zeroed() };
        state.ty = openxr_sys::StructureType::SWAPCHAIN_STATE_FOVEATION_FB;
        state.profile = profile.as_raw();
        // SAFETY: the swapchain is live for as long as this session is, and `state` and `profile` outlive the
        // call - the profile by being kept below, which is what `foveation` is for.
        let result = unsafe {
            (update.update_swapchain)(self.swapchain.as_raw(), &state as *const _ as *const _)
        };
        if result != openxr_sys::Result::SUCCESS {
            log::debug!("wxr-openxr: the swapchain would not take foveation {amount}: {result:?}");
            return;
        }
        self.foveation = Some(profile);
    }
}

impl OpenXrSession {
    pub(super) fn refresh_rates_impl(&mut self, out: &mut Vec<f32>) {
        if !self.refresh_rate {
            return;
        }
        match self.session.enumerate_display_refresh_rates() {
            Ok(rates) => out.extend(rates),
            Err(error) => log::debug!("wxr-openxr: asking for the refresh rates: {error:?}"),
        }
    }
}

impl OpenXrSession {
    pub(super) fn set_refresh_rate_impl(&mut self, rate: f32) {
        if !self.refresh_rate {
            return;
        }
        if let Err(error) = self.session.request_display_refresh_rate(rate) {
            log::debug!("wxr-openxr: asking for {rate} Hz: {error:?}");
        }
    }
}

impl OpenXrSession {
    /// What this instance was made with, as the capability bits the core carries.
    ///
    /// Every bit is an extension that was actually enabled at `xrCreateInstance` or a facility that exists
    /// without one - a bit set for something not enabled would be a promise the session cannot keep, and the
    /// refusals in `layer` and the haptic calls are what an app gets if it asks anyway.
    pub(super) fn features_impl(&self) -> wxr::Features {
        // A quad layer is in the core specification rather than behind an extension, and a swapchain is the
        // only thing one needs - so every session of this backend has a quad, which is the one shape it makes.
        let mut features = wxr::Features::LAYER_QUAD;
        if self.refresh_rate {
            features = features.union(wxr::Features::REFRESH_RATE);
        }
        // One bit per composition-layer extension the instance was made with.
        for (enabled, bit) in [
            (self.layers_enabled.cylinder, wxr::Features::LAYER_CYLINDER),
            (self.layers_enabled.equirect, wxr::Features::LAYER_EQUIRECT),
            (self.layers_enabled.cube, wxr::Features::LAYER_CUBE),
        ] {
            if enabled {
                features = features.union(bit);
            }
        }
        // A hand tracker is the skeleton, so a session that was given one has hand tracking. Everything else
        // this backend asks for - the depth layer, surfaces - it does not get.
        if let Some(hands) = &self.hands
            && hands.has_tracking()
        {
            features = features.union(wxr::Features::HAND_TRACKING);
        }
        // Haptics, and here the bit is the extension rather than a per-controller answer: OpenXR has no way to
        // ask whether a particular controller has an actuator, so a runtime with the extension is a session
        // whose sources say they can buzz, and one without is a session whose sources say they cannot.
        if self.haptics.feedback {
            features = features.union(wxr::Features::HAPTICS);
        }
        features
    }
}
