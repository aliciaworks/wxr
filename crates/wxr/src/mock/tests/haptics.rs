//! Making a controller buzz, and taking the buzz back: what the mock records and what it refuses.

use super::*;

#[test]
fn a_pulse_names_the_source_it_reaches() {
    let mut session = running();
    let space = session.space(SpaceKind::LocalFloor).expect("a floor");
    let mut sources = Vec::new();
    session.inputs(space, &mut sources).unwrap();
    // The controllers have actuators, so this call can be answered; `Ok` alone would not say it moved.
    assert!(sources[0].haptics);
    assert!(session.features().contains(crate::Features::HAPTICS));

    let id = sources[1].id;
    session.pulse(id, 0.5, Duration::from_millis(25)).unwrap();
    // Clamped rather than refused; a NaN is the quietest request there is, which is what the backends send.
    session.pulse(id, 7.0, Duration::from_millis(5)).unwrap();
    session.pulse(id, f32::NAN, Duration::ZERO).unwrap();
    assert_eq!(
        session.pulses(),
        &[
            (id, 0.5, Duration::from_millis(25)),
            (id, 1.0, Duration::from_millis(5)),
            (id, 0.0, Duration::ZERO),
        ]
    );

    session.play_pcm(id, &[0.0, 1.0, 0.0], 800.0).unwrap();
    assert_eq!(session.waveforms(), &[(id, 3, 800.0)]);

    // A stop is a request of its own, not the absence of one: the app knows the buzz should end before its
    // duration does, and the record is what shows it reached the source the buzz did.
    session.stop_pulse(id).unwrap();
    assert_eq!(session.stops(), &[id]);
}

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
    assert!(matches!(
        session.stop_pulse(stranger),
        Err(Error::Unsupported(_))
    ));
    assert!(session.pulses().is_empty(), "and nothing was recorded");
    assert!(session.waveforms().is_empty());
    assert!(session.stops().is_empty());
}
