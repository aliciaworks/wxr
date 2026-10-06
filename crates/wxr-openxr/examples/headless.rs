//! Connect to the runtime the machine has and run a few frames with nothing drawn.
//!
//! This is the seam's proof, and it needs no headset: a wgpu device is made with no window at all, the
//! session is told about it, and the frames come back with two views and an image to draw them into. What a
//! runtime does with a machine that has no display is the runtime's business - Monado has a null device for
//! exactly this - and a `NoDisplay` from `system()` is a machine without a runtime rather than a bug here.

use std::time::Duration;

use wxr::{Backend, Session as _, SpaceKind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A device with no surface: nothing is presented to a window, because the compositor is where the
    // picture goes.
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;
    let (device, _queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("wxr-openxr example"),
        ..Default::default()
    }))?;

    let backend = wxr_openxr::OpenXr::load()?;
    let (min, max) = backend.requirements()?;
    println!(
        "runtime wants a Vulkan device from {min:?} to {max:?}, and recommends {:?} per eye",
        backend.recommended_extent()
    );

    let mut session = backend.connect(wxr_openxr::Device {
        instance: instance.clone(),
        device,
    })?;
    println!(
        "connected: {:?}, presentation {:?}, images {:?}",
        session.state(),
        session.presentation(),
        session.images()
    );

    let space = session.space(SpaceKind::LocalFloor)?;
    let mut frame = wxr::Frame::default();
    let mut rendered = 0;

    for _ in 0..120 {
        while let Some(event) = session.poll() {
            println!("event: {event:?}");
        }
        session.begin(Duration::ZERO, &mut frame)?;
        if !frame.is_render() {
            continue;
        }
        session.views(space, &mut frame)?;
        rendered += 1;

        for view in frame.views() {
            println!(
                "  {:?} at {:?}, fov {:.3}/{:.3}/{:.3}/{:.3}, layer {}",
                view.eye,
                view.pose.position,
                view.fov.up,
                view.fov.down,
                view.fov.left,
                view.fov.right,
                view.layer
            );
        }
        session.end(&mut frame)?;
        if rendered >= 3 {
            break;
        }
    }

    println!("drew {rendered} frames");
    Ok(())
}
