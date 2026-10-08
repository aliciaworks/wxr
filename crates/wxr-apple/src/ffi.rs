//! The C ABI an app's SwiftUI entry calls into.
//!
//! On visionOS an app is Swift up to the point where the compositor hands it a layer renderer, and that
//! handover is the one thing this file exists for: Swift holds a `LayerRenderer` - the `cp_layer_renderer_t`
//! one of these bindings names - and Rust needs the same object retained.
//!
//! A `@_cdecl` boundary
//! carries a pointer and a number and nothing else, so that is what crosses: a pointer in, `0` or `1` out,
//! and a failure is logged rather than thrown, because there is no Swift error on the other side to catch it.
//!
//! The device is adopted here too, because an app is the only place it can be, and the step itself lives in
//! [`crate::compositor`] so that a renderer with a loop of its own takes the same one rather than a copy.
//! This module is what an app gets when it has no such loop: [`crate::entry::run`], the crate's own.
//!
//! [`AppleBackend::device`]: crate::session::AppleBackend::device

use core::ffi::c_void;

/// Run immersive frames until the space closes, called from Swift.
///
/// The app's `CompositorLayer` closure calls this, and the call does not return until the immersive space
/// ends - the loop is [`crate::entry::run`]'s, and stopping is the compositor's own state change.
///
/// # Safety
///
/// `layer_renderer` must be a live `cp_layer_renderer_t`: the pointer Swift hands over for a
/// `CompositorServices.LayerRenderer` while the immersive space is up. It is retained for the duration of the
/// call and released on the way out, so the app keeps its own reference for as long as it needs one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wxr_apple_run(layer_renderer: *mut c_void) -> i32 {
    // SAFETY: the caller's contract says this is a live layer renderer. Retaining gives this call a
    // reference of its own, so the app may drop its own before the loop ends; a null pointer is the one
    // thing the contract does not allow and is caught rather than dereferenced.
    match unsafe { run(layer_renderer) } {
        Ok(()) => 0,
        Err(error) => {
            // Both, deliberately: the `log` facade is the crate's idiom, but an app that installs no
            // logger - which this app is - drops it, and the stderr line is what a `simctl --console-pty`
            // launch shows. Either may be the only record the failure leaves behind.
            log::error!("wxr-apple: the immersive loop ended with an error: {error}");
            eprintln!("wxr-apple: the immersive loop ended with an error: {error}");
            1
        }
    }
}

/// The pointer-taking half of [`wxr_apple_run`], with the error kept as one.
///
/// # Safety
///
/// The same contract as [`wxr_apple_run`]'s `layer_renderer`.
unsafe fn run(layer_renderer: *mut c_void) -> Result<(), wxr::Error> {
    // SAFETY: the same contract as the caller's - this is a live layer renderer.
    let compositor = unsafe { crate::compositor::adopt(layer_renderer)? };

    // The crate's own scene, so that the whole leg is runnable from one call; a renderer with a scene of its
    // own takes [`crate::compositor::adopt`] and drives `wxr::Session` instead, which is what a game does.
    crate::entry::run(
        compositor.renderer,
        compositor.device,
        compositor.queue,
        |_frame, _inputs| {},
    )
}
