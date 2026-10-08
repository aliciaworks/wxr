//! The C ABI an app's SwiftUI entry calls into.
//!
//! On visionOS an app is Swift up to the point where the compositor hands it a layer renderer, and that
//! handover is the one thing this file exists for: Swift holds a `LayerRenderer` - the `cp_layer_renderer_t`
//! one of these bindings names - and Rust needs the same object as a [`Retained`]. A `@_cdecl` boundary
//! carries a pointer and a number and nothing else, so that is what crosses: a pointer in, `0` or `1` out,
//! and a failure is logged rather than thrown, because there is no Swift error on the other side to catch it.
//!
//! The device is made here too, and an app is the only place it can be. The compositor owns the Metal device
//! and wgpu has to be handed one before [`crate::entry::run`] is entered - [`AppleBackend::device`] says why the
//! order is this way and what it costs. On visionOS that device is the only one, so an ordinary
//! `request_adapter` lands on the compositor's own and nothing needs adopting; a platform with more than one
//! device would need the `wgpu-hal` route the same comment describes.
//!
//! [`AppleBackend::device`]: crate::session::AppleBackend::device

use core::ffi::c_void;

use objc2::rc::Retained;
use objc2_compositor_services::cp_layer_renderer_t;

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
    // SAFETY: by the caller's contract the pointer is a live layer renderer, so retaining it is sound; the
    // `Option` is `None` only for a null pointer, which the contract excludes and this returns on.
    let Some(renderer) =
        (unsafe { Retained::retain(layer_renderer.cast::<cp_layer_renderer_t>()) })
    else {
        return Err(wxr::Error::Present(
            "the layer renderer pointer the app passed is null".into(),
        ));
    };

    // The device the compositor draws with, which is the one wgpu has to use - see the module comment. The
    // instance is made without a display handle because there is no window to target: the compositor's own
    // textures are the surfaces, and they are imported rather than presented to.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .map_err(|error| {
                wxr::Error::Present(format!(
                    "no adapter for the compositor's Metal device: {error}"
                ))
            })?;
    // The adapter is the compositor's own device, so what it reports is the ceiling this device has
    // to be opened at. wgpu's default limits are the desktop Metal ones and can sit above it - the
    // simulator reports a lower `max_inter_stage_shader_variables` than the default - and asking for
    // more than the adapter allows is a refused device rather than a clamped one.
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("wxr-apple compositor device"),
        required_limits: adapter.limits(),
        ..Default::default()
    }))
    .map_err(|error| {
        wxr::Error::Present(format!(
            "the compositor's Metal device could not be opened by wgpu: {error}"
        ))
    })?;

    // The crate's own scene, so that the whole leg is runnable from one call; an app with a scene of its own
    // drives `wxr::Session` and does not come through here.
    crate::entry::run(renderer, device, queue, |_frame, _inputs| {})
}
