//! Connecting to whatever this machine has, for the suites that need a runtime.
//!
//! Every test in this directory asks the runtime the same question, so the asking lives here rather than
//! in three files: the loader, the wgpu device with no window behind it, and the lock that stops two of
//! them asking at once.

use std::sync::{Mutex, MutexGuard};

use wxr::Backend as _;

/// One runtime, one instance at a time.
///
/// A loader is loaded once per process and hands out one `XrInstance` at a time; a second one taken while
/// the first lives is not an error but a segmentation fault, which is what the OpenXR loader says in as many
/// words. Rust runs a crate's tests in parallel, so every test takes this lock for as long as its session is
/// alive and they take turns instead. It is recovered from rather than `unwrap`ed because a test that fails
/// while holding it should not turn every later test into a failure about a lock.
pub static RUNTIME: Mutex<()> = Mutex::new(());

/// Connect to whatever this machine has, or say there is nothing to connect to.
///
/// The `None` is a machine with no runtime, which is a fact about the machine rather than a failure of
/// the backend - so a test that gets one asserts nothing and returns.
///
/// What comes back is the runtime's turn, the backend and the session together, and the turn has to be held
/// for as long as the session lives: an instance dropped under a session made from it is the same fault the
/// lock exists to avoid.
pub fn connect() -> Option<(
    MutexGuard<'static, ()>,
    wxr_openxr::OpenXr,
    wxr_openxr::OpenXrSession,
)> {
    let turn = RUNTIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    // The wgpu side has no window: nothing is presented to a display, because the compositor is where the
    // picture goes and this suite never reaches a frame anyway. A device is still needed because a session
    // is asked for one - the seam is that the backend is handed the objects the renderer made.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
        ..Default::default()
    }))
    .ok()?;
    let (device, _queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("wxr-openxr runtime tests"),
        ..Default::default()
    }))
    .ok()?;

    let backend = wxr_openxr::OpenXr::load().ok()?;
    let session = backend
        .connect(
            wxr_openxr::Device {
                instance,
                device: device.clone(),
            },
            wxr::SessionMode::ImmersiveVr,
        )
        .ok()?;
    Some((turn, backend, session))
}
