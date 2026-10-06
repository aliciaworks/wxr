//! Connect to the runtime the machine has and run a few frames with nothing drawn.
//!
//! This is the seam's proof, and it needs no headset: a wgpu device is made with no window at all, the
//! session is told about it, and the frames come back with two views and an image to draw them into. What a
//! runtime does with a machine that has no display is the runtime's business - Monado has a null device for
//! exactly this - and a `NoDisplay` from `system()` is a machine without a runtime rather than a bug here.

#[cfg(not(target_family = "wasm"))]
mod native {
    use std::time::Duration;

    use openxr as xr;
    use wxr::{Backend, Session as _, SpaceKind};

    /// What OpenXR says, reached the way code that only knows the core would: through the seam.
    ///
    /// Nothing on the way in names this backend - the parameter is `impl wxr::Session` - and the core does not
    /// grow a method for it. That is the whole point of the seam: the core is a subset, and this is how a
    /// program asks for the part that was left out.
    fn openxr_state(session: &impl wxr::Session) -> Option<xr::SessionState> {
        session
            .as_backend::<wxr_openxr::OpenXrSession>()
            .map(wxr_openxr::OpenXrSession::openxr_state)
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        // A device with no surface: nothing is presented to a window, because the compositor is where the
        // picture goes.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            ..Default::default()
        }))?;
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
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
            "connected: {:?}, visibility {:?}, presentation {:?}, images {:?}",
            session.state(),
            session.visibility(),
            session.presentation(),
            session.images()
        );
        if let Some(state) = openxr_state(&session) {
            println!("and OpenXR's own state, through the core's seam: {state:?}");
        }

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
}

fn main() {
    #[cfg(not(target_family = "wasm"))]
    native::run().expect("the example failed");
    // The OpenXR backend is native-only, so on a browser there is nothing here to run.
    #[cfg(target_family = "wasm")]
    println!("wxr-openxr: native only; the WebXR backend is `wxr-webxr`");
}
