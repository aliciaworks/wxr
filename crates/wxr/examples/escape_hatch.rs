//! The core's escape hatch, run: `Session::as_backend` hands back the platform underneath.
//!
//! A session is used through the core's vocabulary, and the core says exactly what WebXR says and nothing
//! else. That is what lets three unrelated platforms share it, and the price is that anything only one of
//! them has is not in the trait - so a program that needs OpenXR's own session states, or WebXR's
//! `XRGPUBinding`, or a `cp_drawable`, needs a way back to the type that has them. This is that way, against
//! the mock so that it runs anywhere and so that the example is a check rather than a paragraph.

use wxr::{Backend as _, Session as _};

fn main() {
    let mut session = wxr::mock::MockBackend::default()
        .connect((), wxr::SessionMode::ImmersiveVr)
        .unwrap();
    // The core's vocabulary...
    assert_eq!(session.state(), wxr::State::Connecting);
    // ...and the backend's own type, when the platform is what matters.
    let mock = session.as_backend::<wxr::mock::MockSession>().unwrap();
    assert_eq!(mock.state(), wxr::State::Connecting);
    // And mutably, for the part of a platform a program sets rather than reads.
    assert!(session.as_backend_mut::<wxr::mock::MockSession>().is_some());
    println!("the core's session and the mock session are the same object, seen two ways");
}
