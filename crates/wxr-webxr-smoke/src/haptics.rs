//! The buzz and the buzz taken back, which are two calls and one counter.
//!
//! Kept out of `tick.rs` only to stay under the file-size limit; everything it uses comes from there.

use super::*;

/// How many times the stub has been told to stop a buzz.
fn resets() -> u32 {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("__wxrHapticResets"))
        .ok()
        .and_then(|value| value.as_f64())
        .map(|count| count as u32)
        .unwrap_or(0)
}

/// Ask every source to buzz, then take each buzz back, and check the stub heard both halves.
///
/// The two calls are not independent on a real actuator either: a stop is the same object's `reset`, so the
/// answer to one predicts the answer to the other. A source with no motor refuses both, and a stub that
/// counted a stop it never received is exactly what the counter is for - no frame can see a motor.
pub(super) fn buzz(
    session: &mut wxr_webxr::WebXrSession,
    sources: &[wxr::InputSource],
) -> Result<(), String> {
    let mut buzzing = 0u32;
    for source in sources {
        let sent = match session.pulse(source.id, 0.5, Duration::from_millis(100)) {
            Ok(()) => {
                say(&format!("  {:?} buzz sent", source.handedness));
                true
            }
            Err(error) => {
                say(&format!(
                    "  {:?} would not buzz: {error}",
                    source.handedness
                ));
                false
            }
        };
        if sent {
            buzzing += 1;
        }
        match session.stop_pulse(source.id) {
            Ok(()) if sent => say(&format!("  {:?} buzz stopped", source.handedness)),
            Ok(()) => {
                return Err(format!(
                    "a stop was taken for the {:?} source, which has no motor to stop",
                    source.handedness
                ));
            }
            Err(error) if sent => {
                return Err(format!(
                    "the {:?} source buzzed but would not stop: {error}",
                    source.handedness
                ));
            }
            // The same refusal as the buzz, from the same missing actuator.
            Err(_) => say(&format!(
                "  {:?} would not stop a buzz it never had",
                source.handedness
            )),
        }
    }
    if resets() != buzzing {
        return Err(format!(
            "the stub was told to stop {buzzing} buzzes, and counted {}",
            resets()
        ));
    }
    // A handle this session never handed out is refused here too, and for the reason `pulse` refuses one: a
    // stop that answered `Ok` to a number would be a caller that never learns its id was stale.
    if !matches!(
        session.stop_pulse(wxr::InputId::new(u32::MAX)),
        Err(wxr::Error::Unsupported(_))
    ) {
        return Err("a stop for a source this session never had was answered".to_owned());
    }
    say(
        "a buzz and the stop that ends it early both reached the motor, and a stranger was refused",
    );
    Ok(())
}
