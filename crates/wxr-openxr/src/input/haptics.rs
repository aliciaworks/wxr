//! Haptics: the two ways a controller is made to buzz.
//!
//! Both are the extension's own structures and the same call: `xrApplyHapticFeedback` takes a structure
//! whose *type* says which kind of vibration it is, and the runtime reads the one it knows. `XR_EXT_haptic_feedback`
//! defines the vibration - an amplitude, a frequency and a length - and `XR_FB_haptic_pcm` adds a waveform on
//! top of it, which is why PCM is a second call and not a second meaning for the first.
//!
//! They live apart from the actions that read a controller because they are the one thing here that is
//! *written*: a pose or a button is synced and read once a frame, and a vibration is sent when the app says
//! so and has nothing to read back.

use std::time::Duration;

use openxr as xr;

use crate::Error;

use super::Hands;

impl Hands {
    /// Send a vibration to one source: `intensity` in `0..=1`, for `duration`.
    ///
    /// The frequency is left at zero, which `XR_EXT_haptic_feedback` defines as "whatever the runtime
    /// likes": the core's word is an intensity and a length, and a frequency is a detail only one of the
    /// three backends has a name for.
    pub fn pulse(
        &self,
        session: &xr::Session<xr::Vulkan>,
        source: wxr::InputId,
        intensity: f32,
        duration: Duration,
    ) -> Result<(), Error> {
        let Some((action, path)) = self.output(source) else {
            return Err(Error::Unsupported("haptics on this source".into()));
        };
        // Nanoseconds is what OpenXR counts in, and a `Duration` too long for an `i64` is "as long as
        // possible" rather than an error: the caller asked for a long buzz, and this is the longest there is.
        let nanos = i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX);
        let event = xr::HapticVibration::new()
            .amplitude(intensity.clamp(0.0, 1.0))
            .duration(xr::Duration::from_nanos(nanos))
            .frequency(xr::FREQUENCY_UNSPECIFIED);
        action
            .apply_feedback(session, path, &event)
            .map_err(|error| Error::runtime("send a vibration", error))
    }

    /// Play a waveform on one source, when the runtime took `XR_FB_haptic_pcm`.
    ///
    /// `samples` are amplitudes in `-1..=1` at `sample_rate` hertz. Not looping: an app that wants a held
    /// vibration sends another one, which is the same shape [`Hands::pulse`] has.
    pub fn play_pcm(
        &self,
        session: &xr::Session<xr::Vulkan>,
        source: wxr::InputId,
        samples: &[f32],
        sample_rate: f32,
    ) -> Result<(), Error> {
        if !self.pcm {
            return Err(Error::Unsupported("haptic samples".into()));
        }
        // An empty waveform is nothing to feel, so it is the quiet no-op rather than a runtime call.
        // `XrHapticPcmVibrationFB` valid usage wants at least one sample, and a zero-length buffer is a
        // validation error - or, worse, a runtime that reads a pointer with nothing behind it.
        if samples.is_empty() {
            return Ok(());
        }
        // A rate that is not positive and finite describes no duration: the runtime resamples against it,
        // so there is no honest waveform to hand over.
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(Error::Unsupported(
                "haptic samples at a non-positive rate".into(),
            ));
        }
        let Some((action, path)) = self.output(source) else {
            return Err(Error::Unsupported("haptics on this source".into()));
        };
        // Valid usage requires somewhere for the runtime to write the samples it consumed. The core has
        // nowhere to report that count, but a null pointer is still invalid usage.
        let mut consumed = 0;
        let event = xr::HapticPcmVibrationFB::new()
            .buffer(samples)
            .sample_rate(sample_rate)
            .append(false)
            .samples_consumed(&mut consumed);
        action
            .apply_feedback(session, path, &event)
            .map_err(|error| Error::runtime("play haptic samples", error))
    }

    /// The output action and the path it goes to for a source, if that source has one at all.
    fn output(&self, source: wxr::InputId) -> Option<(&xr::Action<xr::Haptic>, xr::Path)> {
        let hand = self.hands.iter().find(|hand| hand.id == source)?;
        Some((hand.haptic.as_ref()?, hand.haptic_path?))
    }
}
