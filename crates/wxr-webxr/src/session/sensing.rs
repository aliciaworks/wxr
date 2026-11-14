//! What the session itself says: how it reads input, what depth sensing it has, and which features it
//! was granted.
//!
//! These are answers about the session rather than about a frame, which is why they are not in `frames`
//! or `events`. Two of them are questions only a browser can answer - the input of a compositor's session is
//! tracked in the scene by definition, and its depth is a buffer to draw into rather than a measurement the
//! app asked for - and the third is this backend's own reading of what the browser granted.

use super::*;

impl WebXrSession {
    pub(super) fn interaction_mode_impl(&self) -> wxr::InteractionMode {
        let Some(session) = self.inner.borrow().session.clone() else {
            return wxr::InteractionMode::WorldSpace;
        };
        match session.interaction_mode() {
            XrInteractionMode::WorldSpace => wxr::InteractionMode::WorldSpace,
            XrInteractionMode::ScreenSpace => wxr::InteractionMode::ScreenSpace,
            // A browser that says something this build has never heard of: the world is where the input of a
            // session drawn into a display is, and it is the answer for every other backend too.
            XrInteractionMode::__Invalid => wxr::InteractionMode::WorldSpace,
        }
    }
}

impl WebXrSession {
    pub(super) fn depth_sensing_impl(&self) -> Option<wxr::DepthSensing> {
        let session = self.inner.borrow().session.clone()?;
        // When it asked for any, which is what the core's question is: the four attributes below exist on the
        // object either way, and a session that was never granted `depth-sensing` reads them back as values
        // for a measurement it cannot make. `None` is the answer that means that.
        if !has_feature(&session, "depth-sensing") {
            return None;
        }
        Some(wxr::DepthSensing {
            usage: match session.depth_usage() {
                XrDepthUsage::CpuOptimized => wxr::DepthUsage::CpuOptimized,
                XrDepthUsage::GpuOptimized => wxr::DepthUsage::GpuOptimized,
                XrDepthUsage::__Invalid => wxr::DepthUsage::GpuOptimized,
            },
            format: match session.depth_data_format() {
                XrDepthDataFormat::LuminanceAlpha => wxr::DepthFormat::LuminanceAlpha,
                XrDepthDataFormat::Float32 => wxr::DepthFormat::Float32,
                XrDepthDataFormat::UnsignedShort => wxr::DepthFormat::UnsignedShort,
                XrDepthDataFormat::__Invalid => wxr::DepthFormat::Float32,
            },
            ty: session.depth_type().map(|ty| match ty {
                XrDepthType::Raw => wxr::DepthType::Raw,
                XrDepthType::Smooth => wxr::DepthType::Smooth,
                XrDepthType::__Invalid => wxr::DepthType::Raw,
            }),
            active: session.depth_active(),
        })
    }
}

impl WebXrSession {
    /// Start or stop the browser measuring the room.
    ///
    /// The browser's two methods are transitions and not assignments: a `pauseDepthSensing` on a session
    /// that is already paused throws `InvalidStateError`, and so does a resume that has nothing to resume.
    /// That is why the state the browser reports is read first and a request that matches it is answered
    /// without calling anything - a frame loop saying "paused" every frame must not be a frame loop that
    /// fails on the second one.
    ///
    /// A runtime that did not say which state it is in is asked anyway: `depthActive` may be absent, and an
    /// absent answer is not the same as the state asked for.
    pub(super) fn set_depth_active_impl(&mut self, active: bool) -> Result<(), wxr::Error> {
        let Some(session) = self.inner.borrow().session.clone() else {
            return Err(wxr::Error::Unavailable(
                "the session has not started yet".into(),
            ));
        };
        // Granted and not merely present: every session has the two methods, and one that was not made for
        // an AR session calls them into `NotSupportedError`.
        if !has_feature(&session, "depth-sensing") {
            return Err(wxr::Error::Unsupported("depth-sensing".into()));
        }
        if session.depth_active() == Some(active) {
            return Ok(());
        }
        let result = if active {
            throws::resume_depth_sensing(&session)
        } else {
            throws::pause_depth_sensing(&session)
        };
        result.map_err(|error| wxr::Error::Rejected(format!("{error:?}")))
    }

    /// Whether the browser says depth delivery is paused.
    ///
    /// An absent `depthActive` is a runtime that did not say, not one that said no - so a measurement it
    /// delivered is still this frame's measurement.
    pub(super) fn depth_paused(&self) -> bool {
        self.inner
            .borrow()
            .session
            .as_ref()
            .is_some_and(|session| session.depth_active() == Some(false))
    }

    pub(super) fn features_impl(&self) -> wxr::Features {
        let Some(session) = self.inner.borrow().session.clone() else {
            // Before the session arrives there is nothing to ask, and an answer that is not here yet is not a
            // capability.
            return wxr::Features::NONE;
        };
        let mut features = wxr::Features::NONE;
        for (bit, name) in [
            (wxr::Features::PLANES, "plane-detection"),
            (wxr::Features::HIT_TEST, "hit-test"),
            (wxr::Features::LIGHT_ESTIMATION, "light-estimation"),
            (wxr::Features::HAND_TRACKING, "hand-tracking"),
            (wxr::Features::ANCHORS, "anchors"),
        ] {
            if has_feature(&session, name) {
                features = features.union(bit);
            }
        }
        // A session that reports `supportedFrameRates` is one whose display can be asked to run at another
        // rate, and a browser that reports null there is one where the display picks and a page cannot.
        if session.supported_frame_rates().is_some() {
            features = features.union(wxr::Features::REFRESH_RATE);
        }
        // Depth is granted by a feature and readable only through the binding that gives a texture: a browser
        // that granted it and no binding is a depth this backend cannot hand over. The usage matters too,
        // because a CPU-optimised session delivers bytes rather than the texture [`wxr::Session::Depth`] is.
        if has_feature(&session, "depth-sensing")
            && self.gpu.is_some()
            && session.depth_usage() != XrDepthUsage::CpuOptimized
        {
            features = features.union(wxr::Features::DEPTH);
        }
        // Layers need two things and the session has to have both: the binding, which is what makes one, and the
        // feature descriptor, which is what lets a non-projection layer be composited at all. A session with
        // both can make every shape the specification defines, so every shape bit is set rather than a quad's
        // alone.
        if self.gpu.is_some() && has_feature(&session, "layers") {
            features = features
                .union(wxr::Features::LAYER_QUAD)
                .union(wxr::Features::LAYER_CYLINDER)
                .union(wxr::Features::LAYER_EQUIRECT)
                .union(wxr::Features::LAYER_CUBE);
        }
        // Haptics is the gamepad's rather than the session's: an actuator belongs to a controller, so the only
        // place the answer exists is the controllers that are connected. An empty list, and a list of
        // controllers with no actuators, are the same answer - a session with nothing to buzz.
        let held = session.input_sources();
        for index in 0..held.length() {
            let Some(source) = held.get(index) else {
                continue;
            };
            if haptics::actuator(&source).is_some() {
                features = features.union(wxr::Features::HAPTICS);
                break;
            }
        }
        features
    }
}

impl WebXrSession {
    /// The rate the display is running at, which is `XRSession.frameRate`.
    ///
    /// Not the rate the last request asked for: a request is granted on the runtime's own schedule, and this
    /// is the browser's answer to what is actually on the display now. A value that is not finite is not a
    /// rate, so it is answered as no answer rather than passed on.
    pub(super) fn refresh_rate_impl(&self) -> Option<f32> {
        let session = self.inner.borrow().session.clone()?;
        session.frame_rate().filter(|rate| rate.is_finite())
    }
}

impl WebXrSession {
    /// The rates this display offers, appended to `out` as the core's contract says.
    pub(super) fn refresh_rates_impl(&mut self, out: &mut Vec<f32>) {
        let Some(session) = self.inner.borrow().session.clone() else {
            return;
        };
        // `supportedFrameRates` is a `Float32Array?`: null is a display that cannot be asked, and the list is
        // the whole of the answer when it can.
        match session.supported_frame_rates() {
            Some(rates) => out.extend(rates),
            None => log::debug!("wxr-webxr: this display does not let a page choose its rate"),
        }
    }

    /// Ask the display to run at `rate` hertz.
    pub(super) fn set_refresh_rate_impl(&mut self, rate: f32) {
        // `updateTargetFrameRate` takes a WebIDL `float`, and the conversion of a NaN or an infinity is a
        // `TypeError` thrown through the wasm frame rather than a rejected promise. The specification has no
        // range to clamp into - a rate the display does not list is a rejection, not a clamp - so these are
        // not requests at all and leave the display's rate alone.
        if !rate.is_finite() {
            log::debug!("wxr-webxr: no refresh rate for {rate}, which is not a number of hertz");
            return;
        }
        let Some(session) = self.inner.borrow().session.clone() else {
            return;
        };
        // A null list is a display the page cannot choose for, so the request would be one the specification
        // rejects without the display ever hearing it. The rate it has is the answer, and nothing is sent.
        if session.supported_frame_rates().is_none() {
            log::debug!("wxr-webxr: this display does not let a page choose {rate} Hz");
            return;
        }
        let promise = match throws::update_target_frame_rate(&session, rate) {
            Ok(promise) => promise,
            Err(error) => {
                log::debug!("wxr-webxr: asking for {rate} Hz: {error:?}");
                return;
            }
        };
        // The promise is the answer to the request, and either rejection is one: `TypeError` for a rate this
        // display does not offer, `InvalidStateError` for a session that has ended. Read and dropped, because
        // an unhandled rejection on the page is a report of something the app already asked to be a request.
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(error) = JsFuture::from(promise).await {
                log::debug!("wxr-webxr: the display kept its rate: {error:?}");
            }
        });
    }
}
