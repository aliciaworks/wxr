//! What the mock says about a source it never handed out.

use super::*;

#[test]
fn a_pulse_to_a_source_this_session_never_had_is_refused() {
    // A source id is a number, and a caller can keep one past the set `inputs` returned. Every real backend
    // answers `Unsupported` for that, so a mock that recorded the call would let the id ship.
    let mut session = running();
    let stranger = InputId::new(crate::mock::INPUT_COUNT);
    assert!(matches!(
        session.pulse(stranger, 1.0, Duration::from_millis(10)),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        session.play_pcm(stranger, &[1.0, -1.0], 800.0),
        Err(Error::Unsupported(_))
    ));
    assert!(session.pulses().is_empty(), "and nothing was recorded");
    assert!(session.waveforms().is_empty());
}
