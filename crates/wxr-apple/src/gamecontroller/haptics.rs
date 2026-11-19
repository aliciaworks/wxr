//! One pulse: a Core Haptics pattern, and the player that carries it.
//!
//! Apple's haptics are patterns rather than waveforms - a continuous event with an intensity and a length -
//! which is exactly the core's word for a buzz, and the reason `play_pcm` has nothing to become here.

use std::time::Duration;

use objc2::AllocAnyThread;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_core_haptics::{
    CHHapticEngine, CHHapticEvent, CHHapticEventParameter, CHHapticEventParameterIDHapticIntensity,
    CHHapticEventTypeHapticContinuous, CHHapticPattern, CHHapticPatternPlayer,
};
use objc2_foundation_visionos::NSArray;

/// Build the pattern for one pulse and hand back the player that will play it.
///
/// One continuous event whose only parameter is its intensity: the core's word is an intensity and a length,
/// and a continuous event with a duration is exactly that. `HapticSharpness` is left out on purpose - it is
/// the character of a feel rather than its strength, and a core word for it would be one only this platform
/// has a name for.
pub(super) fn pattern(
    engine: &CHHapticEngine,
    intensity: f32,
    duration: Duration,
) -> Result<Retained<ProtocolObject<dyn CHHapticPatternPlayer>>, wxr::Error> {
    // `clamp` passes NaN through, and a parameter that is not a number is a pattern Core Haptics refuses to
    // make - an error out of a call whose only other failures are a framework that is not there. NaN is not
    // an intensity, so it becomes the quiet end of the range, as OpenXR's and WebXR's pulses do; the
    // infinities keep the clamp.
    let intensity = if intensity.is_nan() {
        0.0
    } else {
        intensity.clamp(0.0, 1.0)
    };
    // SAFETY: every object below is the framework's, made here and dropped at the end of the call, and the
    // parameter id and event type are constants.
    unsafe {
        let parameter = CHHapticEventParameter::initWithParameterID_value(
            CHHapticEventParameter::alloc(),
            CHHapticEventParameterIDHapticIntensity,
            intensity,
        );
        let event = CHHapticEvent::initWithEventType_parameters_relativeTime_duration(
            CHHapticEvent::alloc(),
            CHHapticEventTypeHapticContinuous,
            &NSArray::from_retained_slice(std::slice::from_ref(&parameter)),
            0.0,
            duration.as_secs_f64(),
        );
        let pattern = CHHapticPattern::initWithEvents_parameters_error(
            CHHapticPattern::alloc(),
            &NSArray::from_retained_slice(std::slice::from_ref(&event)),
            &NSArray::from_retained_slice(&[]),
        )
        .map_err(|error| wxr::Error::Rejected(format!("{error}").into()))?;
        engine
            .createPlayerWithPattern_error(&pattern)
            .map_err(|error| wxr::Error::Rejected(format!("{error}").into()))
    }
}
