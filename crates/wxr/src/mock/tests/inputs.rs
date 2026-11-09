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

#[test]
fn a_hand_is_answered_for_a_source_that_has_one() {
    // The mock has two hands, so both of its own ids are skeletons - and an id it never handed out is not.
    // The difference is what a caller develops against: a session that answered for every number would
    // let a controller id draw a hand that a real backend leaves empty.
    let mut session = with_hands();
    let space = session.space(SpaceKind::LocalFloor).expect("a floor");
    let mut hand = Hand::default();
    session.hand(InputId::new(0), space, &mut hand).unwrap();
    assert!(hand.is_tracked(), "the first source is a hand");

    let stranger = InputId::new(crate::mock::INPUT_COUNT);
    session.hand(stranger, space, &mut hand).unwrap();
    assert!(!hand.is_tracked(), "and a source past the two is empty");
}
