#![cfg(not(target_family = "wasm"))]

//! What wxr's WebXR vocabulary means, checked against another implementation of the same API.
//!
//! wxr is a WebXR-shaped API with three backends behind it, and the WebXR Device API has more than one
//! implementation to compare against. The one this crate compares with is Servo's - a Rust reading of the
//! same specification, published as `servo-webxr-api` - because it is the one that can be named as a
//! dependency and driven from a test: the browser implementations live in C++ and JavaScript inside
//! repositories that a test here could not build, and their behaviour is reached through a page.
//!
//! The comparison is not of the whole API, and it is not of the platform. What it can check is that the two
//! readings agree about the *vocabulary* - that an eye is the same two eyes, that a session mode names the
//! same thing, that a space kind is the same reference space - because that is what a specification fixes and
//! what a backend cannot be asked about without the hardware. Where a backend *does* the thing - a pose, a
//! frame, an input - the test belongs with that backend, and on the machine that can run it: the visionOS
//! simulator for ARKit, Monado for OpenXR.
