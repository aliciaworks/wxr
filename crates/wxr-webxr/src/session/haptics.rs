//! Haptics: the buzz a controller can be asked for.
//!
//! WebXR has no haptics of its own. A source that can be felt carries a gamepad, and the gamepad carries
//! actuators, so this is where the gamepads module's `hapticActuators` meets the core's call. The one it
//! implements is the Gamepad specification's `pulse` - an intensity and a length, which is the shape every
//! browser that has an actuator has.
//!
//! The draft's `playPCM` is *bound* (`Tools/refresh_webxr_idl.py` generates it) and not called here, and the
//! reason is the argument it takes: an `AudioBuffer` is a Web Audio object, made by an `AudioContext`, and
//! this backend has no audio graph and no reason for one. The samples the core hands over are exactly what an
//! `AudioBuffer` is made of, so the day it is worth it, it is `createBuffer` and a copy - a piece of its own
//! rather than a line here, and one OpenXR's `XR_FB_haptic_pcm` already answers without.

use super::*;

use crate::sys::{GamepadHapticActuator, XrInputSource};

/// The actuator a source buzzes with, if it has one.
///
/// `Gamepad.hapticActuators` is the list and the first is the one every platform that has one puts first: a
/// controller with two motors is a controller with an order, and the specification does not say which motor
/// is which. A source with no gamepad, and a gamepad with no actuators, are both a source that cannot be
/// felt - which is `None`, and what `InputSource::haptics` and `Features::HAPTICS` are both built on.
pub(super) fn actuator(source: &XrInputSource) -> Option<GamepadHapticActuator> {
    let value = source.gamepad();
    if value.is_null_or_undefined() {
        return None;
    }
    let gamepad = value.dyn_into::<web_sys::Gamepad>().ok()?;
    gamepad
        .haptic_actuators()
        .iter()
        .find_map(|value| value.dyn_into::<GamepadHapticActuator>().ok())
}

impl WebXrSession {
    /// `GamepadHapticActuator.pulse`, which the browser measures in milliseconds.
    ///
    /// The promise it returns is dropped rather than awaited: the core's word is a request that has been
    /// sent, and making a frame's timing depend on a controller's would be the tail wagging the dog - the
    /// buzz either happens or it does not, and the frame has moved on either way.
    ///
    /// A source the session has not seen, or one with no actuator, is `Unsupported`: the caller can find out
    /// from `InputSource::haptics` before asking, and an answer it can act on is worth more than a quiet
    /// nothing.
    pub(super) fn pulse_impl(
        &mut self,
        source: wxr::InputId,
        intensity: f32,
        duration: Duration,
    ) -> Result<(), wxr::Error> {
        let input = self
            .sources
            .get(source)
            .ok_or_else(|| wxr::Error::Unsupported("haptics".into()))?;
        let actuator = actuator(&input)
            .ok_or_else(|| wxr::Error::Unsupported("haptics on this source".into()))?;
        let _ = actuator.pulse(
            intensity.clamp(0.0, 1.0) as f64,
            duration.as_millis() as f64,
        );
        Ok(())
    }
}
