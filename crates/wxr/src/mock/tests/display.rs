//! What a session with no optics of its own says about the display it is drawn into.

use super::*;

#[test]
fn a_backend_that_cannot_measure_device_fit_answers_unknown() {
    // Fit is a fact about the wearer and the display's optics, and a fabricated session has neither - so the
    // bit is absent and the answer is `Unknown`. The difference a real backend makes is the bit; the enum
    // word for "no measurement" is the same one an unfitted frame gets.
    let mut session = running();
    assert!(!session.features().contains(Features::DEVICE_FIT));
    assert_eq!(session.device_fit(), crate::DeviceFit::Unknown);
}
