//! The compositor's device, adopted by a renderer.
//!
//! On visionOS the compositor owns the Metal device, because the images it hands out belong to one - see
//! [`AppleBackend::device`](crate::session::AppleBackend::device). A renderer therefore does not make a device;
//! it adopts this one, and every image wrapped from the compositor is a texture of it. The check is not
//! academic: an image imported through a device that did not make it is not the compositor's image, and the
//! compositor will not present it.
//!
//! This is the same step an app takes before [`entry::run`](crate::entry::run), lifted out of the C ABI so
//! that a renderer with a loop of its own - an engine, a game - can take it too. [`adopt`] is unsafe because
//! what it is handed comes from Swift as a pointer and nothing in Rust can check it; everything after that is
//! ordinary.

use core::ffi::c_void;

use objc2::rc::Retained;
use objc2_compositor_services::cp_layer_renderer_t;

/// What the compositor owns, as the renderer's side of it.
///
/// The three are one fact: the device is the layer renderer's device, and the queue is that device's queue -
/// which is also the one presenting commits a command buffer on, so a renderer draws and presents on the same
/// one rather than making a second.
pub struct Compositor {
    /// The layer renderer, retained: every session made from this device is made from it, and it has to stay
    /// alive for as long as the session does.
    pub renderer: Retained<cp_layer_renderer_t>,
    /// The wgpu device on the compositor's own Metal device.
    pub device: wgpu::Device,
    /// Its queue.
    pub queue: wgpu::Queue,
}

/// Adopt the compositor's device from the layer renderer a `CompositorLayer` closure handed over.
///
/// # Safety
///
/// `layer_renderer` must be a live `cp_layer_renderer_t`: the pointer Swift passes for a
/// `CompositorServices.LayerRenderer` while its immersive space is up. It is retained here, so the app's own
/// reference may be dropped afterwards; the pointer is the one thing the contract cannot be checked against and
/// a null one is caught rather than dereferenced.
pub unsafe fn adopt(layer_renderer: *mut c_void) -> Result<Compositor, wxr::Error> {
    // SAFETY: by the caller's contract the pointer is a live layer renderer, so retaining it is sound.
    let Some(renderer) =
        (unsafe { Retained::retain(layer_renderer.cast::<cp_layer_renderer_t>()) })
    else {
        return Err(wxr::Error::Present(
            "the layer renderer pointer the app passed is null".into(),
        ));
    };

    // The instance is made without a display handle because there is no window to target: the compositor's own
    // textures are the surfaces, and they are imported rather than presented to.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    // On visionOS there is a single device, so an ordinary `request_adapter` lands on the compositor's own and
    // nothing needs adopting beyond naming it; `AppleBackend::device` describes the `wgpu-hal` route a platform
    // with more than one device would need.
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .map_err(|error| {
                wxr::Error::Present(format!(
                    "no adapter for the compositor's Metal device: {error}"
                ))
            })?;
    // The adapter is the compositor's own device, so what it reports is the ceiling this device has to be
    // opened at. wgpu's default limits are the desktop Metal ones and can sit above it - the simulator reports a
    // lower `max_inter_stage_shader_variables` than the default - and asking for more than the adapter allows is
    // a refused device rather than a clamped one.
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

    Ok(Compositor {
        renderer,
        device,
        queue,
    })
}
